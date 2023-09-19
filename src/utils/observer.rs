use std::rc::Rc;

pub trait Observer<T> {
    fn update(&self, event: &T);
}

pub trait Observable<T> {
    fn register(&self, observer: Rc<dyn Observer<T>>);
    fn notify(&self, event: T);
}
