use appbase::*;

/*
 * Plugin typename MUST be unique.
 */
pub struct TemplatePlugin {
   /*
    * Plugin SHOULD include `base: PluginBase` as its field.
    */
   base: PluginBase,
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
    * Plugin trait impl SHOULD have `appbase_plugin_default!` macro
    */
   appbase_plugin_default!(TemplatePlugin);

   /*
    * Plugin trait impl MUST implement following methods:
    *    fn new() -> Self;
    *    fn initialize(&mut self);
    *    fn startup(&mut self);
    *    fn shutdown(&mut self);
    *    fn typename() -> String;         // automatically added by appbase_plugin_default!
    *    fn name(&self) -> String;        // automatically added by appbase_plugin_default!
    *    fn state(&self) -> PluginState;  // automatically added by appbase_plugin_default!
    */

   fn new() -> Self {
      TemplatePlugin {
         base: PluginBase::new(),
         // ... other fields, if exist.
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
