use appbase::application::APP;

#[tokio::main]
async fn main() {
   unsafe {
      APP.initialize();
      APP.startup();
      APP.execute().await; // XXX: a better way for graceful shutdown?
   }
}
