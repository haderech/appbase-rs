pub fn current_exe() -> String {
   String::from(std::env::current_exe().unwrap().file_stem().unwrap().to_str().unwrap())
}
