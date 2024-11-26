include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub struct Calculator {
    object: com_example_Calculator,
}

impl Calculator {
    fn new() -> Self {
        let object = com_example_Calculator::new().unwrap();
        Self { object }
    }

    fn add(&self, a: i32, b: i32) -> i32 {
        com_example_Calculator::add(&self.object.inner, a, b).unwrap()
    }

    fn multiply(&self, a: i32, b: i32) -> i32 {
        com_example_Calculator::multiply(&self.object.inner, a, b).unwrap()
    }

    fn test(&self) -> i32 {
        com_example_Calculator::dataHolderTest(&self.object.inner).unwrap()
    }

    fn static_test() -> i32 {
        com_example_Calculator::staticTest().unwrap()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn add() {
        let calculator = super::Calculator::new();
        assert_eq!(calculator.add(1, 2), 3);
    }

    #[test]
    fn multiply() {
        let calculator = super::Calculator::new();
        assert_eq!(calculator.multiply(3, 2), 6);
    }

    #[test]
    fn classes() {
        let calculator = super::Calculator::new();
        assert_eq!(calculator.test(), 1);
    }

    #[test]
    fn static_test() {
        assert_eq!(super::Calculator::static_test(), 1);
    }
}

