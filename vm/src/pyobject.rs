use std::rc::Rc;
use std::fmt;
use super::bytecode;
use super::objint;
use super::objtype;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Add, Mul, Sub};

/* Python objects and references.

Okay, so each python object itself is an class itself (PyObject). Each
python object can have several references to it (PyObjectRef). These
references are Rc (reference counting) rust smart pointers. So when
all references are destroyed, the object itself also can be cleaned up.
Basically reference counting, but then done by rust.

*/

/*
 * Good reference: https://github.com/ProgVal/pythonvm-rust/blob/master/src/objects/mod.rs
 */

/*
The PyRef type implements
https://doc.rust-lang.org/std/cell/index.html#introducing-mutability-inside-of-something-immutable
*/
pub type PyRef<T> = Rc<RefCell<T>>;
pub type PyObjectRef = PyRef<PyObject>;
pub type PyResult = Result<PyObjectRef, PyObjectRef>; // A valid value, or an exception

/*
impl fmt::Display for PyObjectRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Obj {:?}", self)
    }
}*/

#[derive(Debug)]
pub struct PyContext {
    pub type_type: PyObjectRef,
    pub int_type: PyObjectRef,
}

// Basic objects:
impl PyContext {
    pub fn new() -> PyContext {
        let type_type = objtype::create_type();
        let int_type = objint::create_type(type_type.clone());
        // let str_type = objstr::make_type();
        PyContext {
            type_type: type_type,
            int_type: int_type,
        }
    }

    pub fn new_int(&self, i: i32) -> PyObjectRef {
        PyObject::new(PyObjectKind::Integer { value: i }, self.type_type.clone())
    }

    pub fn new_str(&self, s: String) -> PyObjectRef {
        PyObject::new(PyObjectKind::String { value: s }, self.type_type.clone())
    }

    pub fn new_bool(&self, b: bool) -> PyObjectRef {
        PyObject::new(PyObjectKind::Boolean { value: b }, self.type_type.clone())
    }
}

pub trait Executor {
    fn call(&mut self, PyObjectRef) -> PyResult;
    fn new_str(&self, s: String) -> PyObjectRef;
    fn new_bool(&self, b: bool) -> PyObjectRef;
    fn get_none(&self) -> PyObjectRef;
    fn get_type(&self) -> PyObjectRef;
    fn context(&self) -> &PyContext;
}

#[derive(Debug)]
pub struct PyObject {
    pub kind: PyObjectKind,
    pub typ: Option<PyObjectRef>,
    pub dict: HashMap<String, PyObjectRef>, // __dict__ member
}

impl Default for PyObject {
    fn default() -> PyObject {
        PyObject {
            kind: PyObjectKind::None,
            typ: None,
            dict: HashMap::new(),
        }
    }
}

type RustPyFunc = fn(rt: &mut Executor, Vec<PyObjectRef>) -> PyResult;

// #[derive(Debug)]
pub enum PyObjectKind {
    String {
        value: String,
    },
    Integer {
        value: i32,
    },
    Boolean {
        value: bool,
    },
    List {
        elements: Vec<PyObjectRef>,
    },
    Tuple {
        elements: Vec<PyObjectRef>,
    },
    Dict,
    Iterator {
        position: usize,
        iterated_obj: PyObjectRef,
    },
    Slice {
        start: Option<i32>,
        stop: Option<i32>,
        step: Option<i32>,
    },
    NameError {
        // TODO: improve python object and type system
        name: String,
    },
    Code {
        code: bytecode::CodeObject,
    },
    Function {
        code: bytecode::CodeObject,
    },
    Module,
    None,
    Type,
    RustFunction {
        function: RustPyFunc,
    },
}

impl fmt::Debug for PyObjectKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Some kind of python obj")
    }
}

impl PyObject {
    pub fn new(kind: PyObjectKind, typ: PyObjectRef) -> PyObjectRef {
        PyObject {
            kind: kind,
            typ: Some(typ),
            dict: HashMap::new(),
        }.into_ref()
    }

    pub fn call(&self, rt: &mut Executor, args: Vec<PyObjectRef>) -> PyResult {
        match self.kind {
            PyObjectKind::RustFunction { ref function } => function(rt, args),
            _ => {
                println!("Not impl {:?}", self);
                panic!("Not impl");
            }
        }
    }

    pub fn str(&self) -> String {
        match self.kind {
            PyObjectKind::String { ref value } => value.clone(),
            PyObjectKind::Integer { ref value } => format!("{:?}", value),
            PyObjectKind::List { ref elements } => format!(
                "[{}]",
                elements
                    .iter()
                    .map(|elem| elem.borrow_mut().str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            PyObjectKind::Tuple { ref elements } => format!(
                "{{{}}}",
                elements
                    .iter()
                    .map(|elem| elem.borrow_mut().str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            PyObjectKind::None => String::from("None"),
            _ => {
                println!("Not impl {:?}", self);
                panic!("Not impl");
            }
        }
    }

    // Implement iterator protocol:
    pub fn nxt(&mut self) -> Option<PyObjectRef> {
        match self.kind {
            PyObjectKind::Iterator {
                ref mut position,
                iterated_obj: ref iterated_obj_ref,
            } => {
                let iterated_obj = &*iterated_obj_ref.borrow_mut();
                match iterated_obj.kind {
                    PyObjectKind::List { ref elements } => {
                        if *position < elements.len() {
                            let obj_ref = elements[*position].clone();
                            *position += 1;
                            Some(obj_ref)
                        } else {
                            None
                        }
                    }
                    _ => {
                        panic!("NOT IMPL");
                    }
                }
            }
            _ => {
                panic!("NOT IMPL");
            }
        }
    }

    // Move this object into a reference object, transferring ownership.
    pub fn into_ref(self) -> PyObjectRef {
        Rc::new(RefCell::new(self))
    }

}

impl<'a> Add<&'a PyObject> for &'a PyObject {
    type Output = PyObjectKind;

    fn add(self, rhs: &'a PyObject) -> Self::Output {
        match self.kind {
            PyObjectKind::Integer { value: ref value1 } => match &rhs.kind {
                PyObjectKind::Integer { value: ref value2 } => PyObjectKind::Integer {
                    value: value1 + value2,
                },
                _ => {
                    panic!("NOT IMPL");
                }
            },
            PyObjectKind::String { value: ref value1 } => match rhs.kind {
                PyObjectKind::String { value: ref value2 } => PyObjectKind::String {
                    value: format!("{}{}", value1, value2),
                },
                _ => {
                    panic!("NOT IMPL");
                }
            },
            _ => {
                // TODO: Lookup __add__ method in dictionary?
                panic!("NOT IMPL");
            }
        }
    }
}

impl<'a> Sub<&'a PyObject> for &'a PyObject {
    type Output = PyObjectKind;

    fn sub(self, rhs: &'a PyObject) -> Self::Output {
        match self.kind {
            PyObjectKind::Integer { value: value1 } => match rhs.kind {
                PyObjectKind::Integer { value: value2 } => PyObjectKind::Integer {
                    value: value1 - value2,
                },
                _ => {
                    panic!("NOT IMPL");
                }
            },
            _ => {
                panic!("NOT IMPL");
            }
        }
    }
}

impl<'a> Mul<&'a PyObject> for &'a PyObject {
    type Output = PyObjectKind;

    fn mul(self, rhs: &'a PyObject) -> Self::Output {
        match self.kind {
            PyObjectKind::Integer { value: value1 } => match rhs.kind {
                PyObjectKind::Integer { value: value2 } => PyObjectKind::Integer {
                    value: value1 * value2,
                },
                _ => {
                    panic!("NOT IMPL");
                }
            },
            PyObjectKind::String { value: ref value1 } => match rhs.kind {
                PyObjectKind::Integer { value: value2 } => {
                    let mut result = String::new();
                    for _x in 0..value2 {
                        result.push_str(value1.as_str());
                    }
                    PyObjectKind::String { value: result }
                }
                _ => {
                    panic!("NOT IMPL");
                }
            },
            _ => {
                panic!("NOT IMPL");
            }
        }
    }
}

// impl<'a> PartialEq<&'a PyObject> for &'a PyObject {
impl PartialEq for PyObject {
    fn eq(&self, other: &PyObject) -> bool {
        match (&self.kind, &other.kind) {
            (
                PyObjectKind::Integer { value: ref v1i },
                PyObjectKind::Integer { value: ref v2i },
            ) => v2i == v1i,
            (PyObjectKind::String { value: ref v1i }, PyObjectKind::String { value: ref v2i }) => {
                *v2i == *v1i
            }
            /*
            (&NativeType::Float(ref v1f), &NativeType::Float(ref v2f)) => {
                curr_frame.stack.push(Rc::new(NativeType::Boolean(v2f == v1f)));
            },
            (&NativeType::Str(ref v1s), &NativeType::Str(ref v2s)) => {
                curr_frame.stack.push(Rc::new(NativeType::Boolean(v2s == v1s)));
            },
            (&NativeType::Int(ref v1i), &NativeType::Float(ref v2f)) => {
                curr_frame.stack.push(Rc::new(NativeType::Boolean(v2f == &(*v1i as f64))));
            },
            (&NativeType::List(ref l1), &NativeType::List(ref l2)) => {
                curr_frame.stack.push(Rc::new(NativeType::Boolean(l2 == l1)));
            },
            */
            _ => panic!(
                "TypeError in COMPARE_OP: can't compare {:?} with {:?}",
                self, other
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PyObjectKind, PyContext};

    #[test]
    fn test_add_py_integers() {
        let ctx = PyContext::new();
        let a = ctx.new_int(33);
        let b = ctx.new_int(12);
        let c = &*a.borrow() + &*b.borrow();
        match c {
            PyObjectKind::Integer { value } => assert_eq!(value, 45),
            _ => assert!(false),
        }
    }

    #[test]
    fn test_multiply_str() {
        let ctx = PyContext::new();
        let a = ctx.new_str(String::from("Hello "));
        let b = ctx.new_int(4);
        let c = &*a.borrow() * &*b.borrow();
        match c {
            PyObjectKind::String { value } => {
                assert_eq!(value, String::from("Hello Hello Hello Hello "))
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn test_type_type() {
        let ctx = PyContext::new();
    }
}