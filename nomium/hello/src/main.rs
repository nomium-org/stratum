use hello::say_hello;
use hello::init_logger;

fn main() {
    init_logger();
    say_hello();
}