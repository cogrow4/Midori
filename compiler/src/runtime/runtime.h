#ifndef MIDORI_RUNTIME_H
#define MIDORI_RUNTIME_H

#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <math.h>

/* --- Basic Types --- */
typedef int64_t  mi_int;
typedef double   mi_float;
typedef int8_t   mi_bool;
typedef char     mi_char;

#define MI_TRUE  1
#define MI_FALSE 0
#define MI_INF   INFINITY
#define MI_NAN   NAN

/* --- Nil type --- */
typedef char mi_nil_t;
#define mi_nil() ((mi_nil_t)0)

/* --- String Type --- */
typedef struct {
    char* data;
    mi_int len;
    mi_int cap;
} mi_str;

mi_str   mi_str_lit(const char* s);
mi_str   mi_str_empty(void);
mi_str   mi_str_new(const char* s);
mi_str   mi_str_from(char* data, mi_int len, mi_int cap);
void     mi_str_free(mi_str* s);
mi_str   mi_str_clone(mi_str s);
mi_int   mi_str_len(mi_str s);
mi_bool  mi_str_eq(mi_str a, mi_str b);
mi_bool  mi_str_ne(mi_str a, mi_str b);
mi_bool  mi_str_lt(mi_str a, mi_str b);
mi_bool  mi_str_le(mi_str a, mi_str b);
mi_bool  mi_str_gt(mi_str a, mi_str b);
mi_bool  mi_str_ge(mi_str a, mi_str b);
mi_str   mi_str_concat(mi_str a, mi_str b);
mi_char  mi_str_at(mi_str s, mi_int i);

/* --- Array Type --- */
typedef struct {
    void* data;
    mi_int len;
    mi_int cap;
    mi_int elem_size;
} mi_array;

mi_array mi_array_new(mi_int elem_size, mi_int cap);
void     mi_array_free(mi_array* arr);
mi_int   mi_array_len(mi_array* arr);
void*    mi_array_get_ptr(mi_array* arr, mi_int i);
mi_int   mi_array_get_int(mi_array* arr, mi_int i);
void     mi_array_set(mi_array* arr, mi_int i, void* val);
void     mi_array_push(mi_array* arr, void* val);
mi_array mi_array_from_lit(void* data, mi_int len, mi_int elem_size);
mi_str   mi_array_to_string(mi_array* arr, mi_str (*elem_to_str)(void*));

/* --- Built-in Output Functions --- */
void mi_print_int(mi_int n);
void mi_print_float(mi_float f);
void mi_print_bool(mi_bool b);
void mi_print_str(mi_str s);
void mi_println_str(mi_str s);
void mi_print(mi_str s);
void mi_println(mi_str s);

/* --- Conversion Helpers --- */
mi_str mi_to_string(mi_int n);
mi_str mi_float_to_string(mi_float f);
mi_int mi_from_string(mi_str s);
mi_str mi_bool_to_string(mi_bool b);

/* --- Input --- */
mi_str mi_read_line(void);

/* --- Comparison (for codegen: eq/ne/lt/le/gt/ge dispatch) --- */
mi_bool mi_int_eq(mi_int a, mi_int b);
mi_bool mi_float_eq(mi_float a, mi_float b);

/* === MATH LIBRARY === */
mi_float mi_pi(void);
mi_float mi_e(void);
mi_float mi_sin(mi_float x);
mi_float mi_cos(mi_float x);
mi_float mi_tan(mi_float x);
mi_float mi_sqrt(mi_float x);
mi_float mi_pow(mi_float x, mi_float y);
mi_float mi_exp(mi_float x);
mi_float mi_log(mi_float x);
mi_float mi_log10(mi_float x);
mi_float mi_abs(mi_float x);
mi_float mi_floor(mi_float x);
mi_float mi_ceil(mi_float x);
mi_float mi_round(mi_float x);
mi_float mi_atan2(mi_float y, mi_float x);

/* === FILE I/O === */
mi_str   mi_read_file(mi_str path);
mi_bool  mi_write_file(mi_str path, mi_str content);
mi_bool  mi_file_exists(mi_str path);

/* === MEMORY ARENA (simplified leak management) === */
typedef struct mi_arena mi_arena;
mi_arena* mi_arena_new(void);
void*     mi_arena_alloc(mi_arena* a, mi_int size);
void      mi_arena_free_all(mi_arena* a);
void      mi_arena_free(mi_arena* a);

/* === STRING BUILDER === */
typedef struct {
    mi_str buf;
} mi_str_builder;

mi_str_builder mi_sb_new(void);
void           mi_sb_append(mi_str_builder* sb, mi_str s);
void           mi_sb_append_cstr(mi_str_builder* sb, const char* s);
void           mi_sb_append_int(mi_str_builder* sb, mi_int n);
void           mi_sb_append_float(mi_str_builder* sb, mi_float f);
mi_str         mi_sb_build(mi_str_builder* sb);
void           mi_sb_free(mi_str_builder* sb);

/* --- Convenience Wrappers (must come after declarations) --- */
static inline mi_nil_t print(mi_str s)     { mi_print(s); return mi_nil(); }
static inline mi_nil_t println(mi_str s)   { mi_println(s); return mi_nil(); }
static inline mi_str   str(mi_int n)       { return mi_to_string(n); }
static inline mi_int   len(mi_array arr)  { return mi_array_len(&arr); }
static inline mi_int   len_str(mi_str s)   { return s.len; }
static inline mi_str   str_bool(mi_bool b) { return mi_bool_to_string(b); }
static inline mi_str   str_char(mi_char c)   { char buf[2] = {c, 0}; return mi_str_lit(buf); }
static inline mi_str   str_float(mi_float f) { return mi_float_to_string(f); }
static inline mi_float pi(void)    { return mi_pi(); }
static inline mi_float e(void)     { return mi_e(); }
static inline mi_str  read_file(mi_str path)    { return mi_read_file(path); }
static inline mi_bool write_file(mi_str path, mi_str content) { return mi_write_file(path, content); }
static inline mi_bool file_exists(mi_str path)  { return mi_file_exists(path); }
/* CLI args */
void     mi_init_args(int argc, char** argv);
mi_array os_args(void);
#endif /* MIDORI_RUNTIME_H */
