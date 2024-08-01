


//Structure for reporting errors and warnings
#[derive(Debug, Clone, PartialEq)]
pub struct Reporting {
    pub status: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl Reporting {
    pub fn new() -> Reporting {
        Reporting {
            status: false,
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn reportError(&mut self, message: String) {
        self.errors.push(message.clone());
        self.status = true;
    }

    pub fn reportWarning(&mut self, message: String) {
        self.warnings.push(message.clone());
    }
}

impl std::fmt::Display for Reporting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Errors: {:?}, Warnings: {:?}", self.errors, self.warnings)
    }
}