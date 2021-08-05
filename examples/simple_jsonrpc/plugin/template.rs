use appbase::*;

/*
 * Plugin typename MUST be unique.
 */
pub struct TemplatePlugin {
}

/*
 * Plugin SHOULD have `appbase_plugin_requires!` macro including dependencies.
 * (case 1) Plugin A without any dependencies: `appbase_plugin_requires!(A; );`
 * (case 2) Plugin A depends on Plugin B and C: `appbase_plugin_requires!(A; B, C);`
 */
appbase_plugin_requires!(TemplatePlugin; );

/*
 * Plugin impl MAY have plugin-specific methods.
 */
impl TemplatePlugin {}

/*
 * Plugin MUST implement `Plugin` trait.
 */
impl Plugin for TemplatePlugin {
   /*
    * Plugin trait impl MUST implement following methods:
    *    fn new() -> Self;
    *    fn initialize(&mut self);
    *    fn startup(&mut self);
    *    fn shutdown(&mut self);
    */

   fn new() -> Self {
      TemplatePlugin {
      }
   }

   fn initialize(&mut self) {
      // ... do initialization
   }

   fn startup(&mut self) {
      // ... do startup
   }

   fn shutdown(&mut self) {
      // ... do shutdown
   }
}
