pub struct Scoped<T> {
    pub scope: String,
    pub value: T,
}

impl<T> Scoped<T> {
    pub fn new(value: T, scope: &str) -> Self {
        Self {
            scope: scope.to_owned(),
            value,
        }
    }
}

impl<T: Clone> Scoped<&T> {
    pub fn owned(&self) -> Scoped<T> {
        Scoped::new(self.value.clone(), &self.scope)
    }
}

pub trait AsScoped {
    fn as_scoped(&self, scope: &str) -> Scoped<&Self> {
        Scoped::new(self, scope)
    }
}
impl<T> AsScoped for T {}
