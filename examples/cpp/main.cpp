#include <iostream>
#include <solid_euclid.h>

int main() {
    char* message = hello_c();
    std::cout << message << std::endl;
    hello_c_free(message);
    return 0;
}
