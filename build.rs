fn main() {
    #[cfg(feature = "dev")]
    {
        use cargo_husky::*;
        let _ = husky_check();
    }
}

