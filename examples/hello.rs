// A simple Rust program demonstrating features the transpiler should handle.

struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn distance_squared(&self, other: &Point) -> i32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
}

fn make_point(x: i32, y: i32) -> Box<Point> {
    Box::new(Point::new(x, y))
}

fn main() {
    let p = make_point(3, 4);
    let nums = vec![1, 2, 3, 4, 5];
    let msg = String::from("Hello, Transpiler!");
    println!("{}", msg);
}
