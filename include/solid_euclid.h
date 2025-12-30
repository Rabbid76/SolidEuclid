#ifndef SOLID_EUCLID_H
#define SOLID_EUCLID_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Returns a C-compatible, heap-allocated string with the hello message.
 *
 * The caller takes ownership of the returned pointer and is responsible for
 * freeing it exactly once by calling hello_c_free(). The pointer must not be
 * freed with free() or any deallocation function other than hello_c_free().
 */
char* hello_c(void);

/**
 * Frees a string allocated by hello_c().
 *
 * The argument must be a pointer previously returned by hello_c() that has
 * not already been freed. Passing any other pointer value, including NULL
 * or a pointer that has already been freed, results in undefined behavior.
 */
void hello_c_free(char* s);

#ifdef __cplusplus
}
#endif

#endif /* SOLID_EUCLID_H */
