use downcast_rs::{impl_downcast, Downcast};

pub trait Base {
	fn type_name() -> &'static str
	where
		Self: Sized;
	fn resolve_deps(&self);
	fn _init(&mut self);
	fn _startup(&mut self);
	fn _shutdown(&mut self);
}

pub trait Plugin: Downcast + Send + Sync + Base {
	fn new() -> Self
	where
		Self: Sized;
	fn init(&mut self) {}
	fn startup(&mut self) {}
	fn shutdown(&mut self) {}
}
impl_downcast!(Plugin);

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum State {
	Registered,
	Initialized,
	Started,
	Stopped,
}
