#[derive(Clone)]
pub struct Scoped<T: Clone> {
    pub scope: String,
    pub value: T,
}

impl<T: Clone> Scoped<T> {
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

pub trait ToScoped {
    fn to_scoped(self, scope: &str) -> Scoped<Self>
    where
        Self: Sized + Clone,
    {
        Scoped::new(self, scope)
    }
}
impl<T> ToScoped for T {}
