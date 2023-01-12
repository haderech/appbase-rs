use std::{
	collections::HashMap,
	sync::{
		atomic::{AtomicBool, Ordering},
		Arc, Mutex, RwLock,
	},
};

use atomic::Atomic;
use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

use channels::Channels;
use options::Options;

use crate::{
	plugin::{Plugin, State},
	util::current_exe,
};

pub mod channels;
pub mod options;

pub static APP: Lazy<App> = Lazy::new(|| App::new());

struct RegisteredPlugin {
	inner: Mutex<Option<Box<dyn Plugin>>>,
	state: Atomic<State>,
}

pub struct App<'a> {
	runtime: RwLock<Option<Runtime>>,
	plugins: RwLock<HashMap<&'static str, RegisteredPlugin>>,
	running_plugins: Mutex<Vec<&'static str>>,
	pub channels: Channels,
	pub options: Options<'a>,
	is_quitting: Arc<AtomicBool>,
}

impl<'a> App<'a> {
	pub fn new() -> Self {
		App {
			runtime: RwLock::new(None),
			plugins: RwLock::new(HashMap::new()),
			running_plugins: Mutex::new(Vec::new()),
			channels: Channels::new(),
			is_quitting: Arc::new(AtomicBool::new(false)),
			options: Options::new(&current_exe()),
		}
	}

	pub fn is_registered<P: Plugin>(&self) -> bool {
		self.plugins.try_read().unwrap().contains_key(P::type_name())
	}

	pub fn register<P: Plugin>(&self) {
		if self.is_registered::<P>() {
			return
		}
		// Prevent repeated registration by cyclic dependency
		self.plugins.try_write().unwrap().insert(
			P::type_name(),
			RegisteredPlugin { inner: Mutex::new(None), state: Atomic::new(State::Registered) },
		);
		let p = Box::new(P::new());
		p.resolve_deps();
		let ps = self.plugins.try_read().expect("locked: plugins");
		let rp = ps.get(P::type_name()).unwrap();
		rp.inner.try_lock().unwrap().replace(p);
	}

	fn plugin_init_by_name(&self, name: &str) {
		(!self.options.is_parsed()).then(|| self.init());
		let ps = self.plugins.try_read().expect("locked: plugins");
		let rp = ps.get(name).unwrap();
		rp.inner.try_lock().unwrap().as_mut().unwrap()._init();
	}

	pub fn plugin_init<P: Plugin>(&self) {
		self.plugin_init_by_name(P::type_name());
	}

	pub fn plugin_startup<P: Plugin>(&self) {
		let ps = self.plugins.try_read().expect("locked: plugins");
		let rp = ps.get(P::type_name()).unwrap();
		rp.inner.try_lock().unwrap().as_mut().unwrap()._startup();
	}

	pub fn init(&self) {
		self.options.parse();

		let mut runtime = self.runtime.try_write().unwrap();
		let mut builder = Builder::new_multi_thread();

		self.options
			.value_of_t::<usize>("app::worker-threads")
			.map(|wt| builder.worker_threads(wt));
		self.options
			.value_of_t::<usize>("app::max-blocking-threads")
			.map(|mbt| builder.max_blocking_threads(mbt));

		runtime.replace(builder.enable_all().build().unwrap());

		self.options
			.value_of_t::<usize>("app::channel-capacity")
			.map(|cc| self.channels.set_capacity(cc));
		self.options.values_of("app::plugin").map(|ps| {
			for p in ps {
				self.plugin_init_by_name(&p);
			}
		});
	}

	pub fn startup(&self) {
		if self.is_quitting.load(Ordering::Acquire) {
			log::warn!("cannot start closing app...");
			return
		}
		for (_, rp) in self.plugins.try_read().unwrap().iter() {
			if rp.state.load(Ordering::Acquire) == State::Initialized {
				rp.inner.try_lock().unwrap().as_mut().unwrap()._startup();
			}
		}
	}

	pub fn execute(&self) {
		if self.running_plugins.try_lock().unwrap().len() == 0 {
			return
		}
		self.runtime.try_read().unwrap().as_ref().unwrap().block_on(async {
			use tokio::signal::unix::*;

			let mut sigint = signal(SignalKind::interrupt()).unwrap();
			let mut sigterm = signal(SignalKind::terminate()).unwrap();

			let is_quitting = self.is_quitting.clone();
			let main_loop =
				self.runtime.try_read().unwrap().as_ref().unwrap().spawn_blocking(move || loop {
					if is_quitting.load(Ordering::Acquire) {
						break
					}
					std::thread::sleep(std::time::Duration::from_secs(1));
				});
			tokio::select! {
			   _ = sigint.recv() => {},
			   _ = sigterm.recv() => {},
			   _ = main_loop => {},
			}
		});
		self.shutdown();
	}

	pub fn quit(&self) {
		self.is_quitting.store(true, Ordering::Release);
	}

	fn shutdown(&self) {
		self.quit();
		self.runtime.try_read().unwrap().as_ref().unwrap().block_on(async {
			log::info!("try graceful shutdown...");
			while Arc::strong_count(&self.is_quitting) > 1 {}
			for name in self.running_plugins.try_lock().unwrap().iter().rev() {
				if let Some(p) = self
					.plugins
					.try_read()
					.unwrap()
					.get(name)
					.unwrap()
					.inner
					.try_lock()
					.unwrap()
					.as_mut()
				{
					p._shutdown();
				};
			}
		});
	}

	pub fn run_with<P: Plugin, R, F>(&self, f: F) -> R
	where
		F: FnOnce(&mut P) -> R,
	{
		let ps = self.plugins.try_read().expect("locked: plugins");
		let mut p = ps.get(P::type_name()).unwrap().inner.try_lock().unwrap();
		f(p.as_mut().unwrap().downcast_mut::<P>().unwrap())
	}

	pub fn state_of<P: Plugin>(&self) -> State {
		self.plugins
			.try_read()
			.unwrap()
			.get(P::type_name())
			.map(|rp| rp.state.load(Ordering::Acquire))
			.unwrap()
	}

	pub fn set_state_of<P: Plugin>(&self, state: State) {
		self.plugins
			.try_read()
			.unwrap()
			.get(P::type_name())
			.map(|rp| rp.state.store(state, Ordering::Release));
		if state == State::Started {
			self.running_plugins.try_lock().unwrap().push(P::type_name());
		}
	}

	pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
	where
		F: std::future::Future + Send + 'static,
		F::Output: Send + 'static,
	{
		self.runtime.try_read().unwrap().as_ref().unwrap().spawn(future)
	}

	pub fn spawn_blocking<F, R>(&self, func: F) -> tokio::task::JoinHandle<R>
	where
		F: FnOnce() -> R + Send + 'static,
		R: Send + 'static,
	{
		self.runtime.try_read().unwrap().as_ref().unwrap().spawn_blocking(func)
	}

	pub fn quit_handle(&self) -> Option<QuitHandle> {
		(!self.is_quitting.load(Ordering::Acquire))
			.then_some(QuitHandle { is_quitting: Some(self.is_quitting.clone()) })
	}
}

pub struct QuitHandle {
	is_quitting: Option<Arc<AtomicBool>>,
}

impl QuitHandle {
	pub fn is_quitting(&self) -> bool {
		match &self.is_quitting {
			Some(q) => q.load(Ordering::Acquire),
			None => true,
		}
	}
	pub fn quit(&mut self) {
		self.is_quitting.take().map(|q| q.store(true, Ordering::Release));
	}
}

impl Drop for QuitHandle {
	fn drop(&mut self) {
		self.is_quitting.take();
	}
}
