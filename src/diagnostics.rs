#[derive(Debug, Clone)]
pub struct Diagnostics {
    verbose: bool,
}

impl Diagnostics {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn log(&self, message: impl AsRef<str>) {
        if self.verbose {
            eprintln!("[verbose] {}", message.as_ref());
        }
    }
}
