#[derive(Debug, Default)]
pub struct Dump {
    errors: Option<syn::Error>,
}

impl Dump {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append(&mut self, error: syn::Error) {
        match self.errors.as_mut() {
            Some(stored) => stored.combine(error),
            None => self.errors = Some(error),
        }
    }

    pub fn errors(&self) -> Option<&syn::Error> {
        self.errors.as_ref()
    }

    pub fn into_errors(self) -> Option<syn::Error> {
        self.errors
    }
}
