use std::rc::Rc;
use std::cell::RefCell;

struct Signal<T> {
    value: Rc<RefCell<T>>,
    deps: Rc<RefCell<Vec<Box<dyn Fn()>>>>,
}

impl<T> Signal<T> {
    fn new(value: T) -> (impl Fn() -> T + Clone, impl Fn(T)) {
        let sig = Signal {
            value: Rc::new(RefCell::new(value)),
            deps: Rc::new(RefCell::new(Vec::new())),
        };
        let getter = {
            let value = Rc::clone(&sig.value);
            move || value.borrow().clone()
        };
        let setter = {
            let value = Rc::clone(&sig.value);
            let deps = Rc::clone(&sig.deps);
            move |new_value| {
                *value.borrow_mut() = new_value;
                for dep in deps.borrow().iter() {
                    dep();
                }
            }
        };
        (getter, setter)
    }

    fn track(&self, dep: impl Fn() + 'static) {
        self.deps.borrow_mut().push(Box::new(dep));
    }
}

thread_local! {
    static CURRENT_DEP: RefCell<Option<Rc<RefCell<dyn Fn()>>>> = RefCell::new(None);
}

fn with_reactive_context<F: FnOnce() + 'static>(f: F) {
    let dep = Rc::new(RefCell::new(f));
    CURRENT_DEP.with(|current| {
        *current.borrow_mut() = Some(Rc::clone(&dep));
        (dep.borrow())();
        *current.borrow_mut() = None;
    });
}

impl<T> Signal<T> {
    fn get(&self) -> T
        where
            T: Clone,
    {
        CURRENT_DEP.with(|current| {
            if let Some(dep) = &*current.borrow() {
                self.track(|| {
                    (dep.borrow())();
                });
            }
        });
        self.value.borrow().clone()
    }

    fn set(&self, new_value: T)
        where
            T: Clone,
    {
        self.value.borrow_mut().replace(new_value);
        for dep in self.deps.borrow().iter() {
            dep();
        }
    }
}

macro_rules! reactive {
    ($fn_name: ident, $body: block) => {
        fn $fn_name() {
            with_reactive_context(move || $body);
        }
    };
}

fn main() {
    let (count, set_count) = Signal::new(0);

    reactive!(root, {
        pritnln!("[ROOT] count: {}", count.get());
        counter(|| count());
    });

    reactive!(child, {
        println!("[CHILD] count: {}", count.get());
    });

    root();

    set_count(1);
}
