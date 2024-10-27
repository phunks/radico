#[macro_export]
macro_rules! hashmap {
    ($( $key: expr => $val: expr ),*) => {
        let mut map = ::std::collections::HashMap::new();
        $(map.insert($key, $val);)*
        map
    }
}

#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => (#[cfg(debug_assertions)] println!($($arg)*));
}

#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[macro_export]
macro_rules! lazy_regex {
    ($($x:ident:$y:tt),*) => {
        $(static $x : LazyLock<Regex> = LazyLock::new(|| Regex::new($y).unwrap());)*};
}