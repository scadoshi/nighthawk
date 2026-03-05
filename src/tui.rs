const HR_CHAR: &str = "-";
const HR_WIDTH: usize = 50;

pub fn welcome() {
    println!("Welcome to nighthawk");
    println!("Try the following commands");
    hr();
    println!("set <key> <value>");
    println!("get <key>");
    println!("delete <key>");
    hr();
}

pub fn hr() {
    println!("{}", HR_CHAR.repeat(HR_WIDTH));
}
