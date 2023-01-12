use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn appbase_plugin(attr: TokenStream, item: TokenStream) -> TokenStream {
	let deps = syn::parse_macro_input!(attr as syn::AttributeArgs);
	let item = syn::parse_macro_input!(item as syn::ItemStruct);

	let name = item.ident.clone();

	let result = quote! {
	   #item

	   impl ::appbase::plugin::Base for #name {
		  fn type_name() -> &'static str {
			 stringify!(#name)
		  }

		  fn resolve_deps(&self) {
			 #(::appbase::app::APP.register::<#deps>();)*
		  }

		  fn _init(&mut self) {
			 if ::appbase::app::APP.state_of::<Self>() != ::appbase::plugin::State::Registered {
				return;
			 } else {
				::appbase::app::APP.set_state_of::<Self>(::appbase::plugin::State::Initialized);
			 }
			 #(::appbase::app::APP.plugin_init::<#deps>();)*
			 self.init();
			 log::info!("initialized");
		  }

		  fn _startup(&mut self) {
			 if ::appbase::app::APP.state_of::<Self>() != ::appbase::plugin::State::Initialized {
				return;
			 } else {
				::appbase::app::APP.set_state_of::<Self>(::appbase::plugin::State::Started);
			 }
			 #(::appbase::app::APP.plugin_startup::<#deps>();)*
			 self.startup();
			 log::info!("started");
		  }

		  fn _shutdown(&mut self) {
			 if ::appbase::app::APP.state_of::<Self>() != ::appbase::plugin::State::Started {
				return;
			 } else {
				::appbase::app::APP.set_state_of::<Self>(::appbase::plugin::State::Stopped);
			 }
			 self.shutdown();
			 log::info!("stopped");
		  }
	   }
	};

	result.into()
}
