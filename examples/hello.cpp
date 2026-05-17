#include <vector>
#include <string>
#include <memory>
#include <iostream>

// A simple C++ class demonstrating features the transpiler should handle.
class Point {
public:
    int x;
    int y;

    Point(int x, int y) : x(x), y(y) {}

    int distance_squared(const Point& other) const {
        int dx = x - other.x;
        int dy = y - other.y;
        return dx * dx + dy * dy;
    }
};

std::unique_ptr<Point> make_point(int x, int y) {
    return std::make_unique<Point>(x, y);
}

int main() {
    auto p = make_point(3, 4);
    std::vector<int> nums = {1, 2, 3, 4, 5};
    std::string msg = "Hello, Transpiler!";
    std::cout << msg << std::endl;
    return 0;
}
