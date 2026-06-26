#include "midori_runtime.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <math.h>
#include <float.h>

/* === String Implementation === */

mi_str mi_str_lit(const char* s) {
    mi_int len = (mi_int)strlen(s);
    char* data = (char*)malloc((size_t)len + 1);
    memcpy(data, s, (size_t)len + 1);
    return (mi_str){ data, len, len };
}

mi_str mi_str_empty(void) {
    char* data = (char*)malloc(1);
    data[0] = '\0';
    return (mi_str){ data, 0, 0 };
}

mi_str mi_str_new(const char* s) {
    mi_int len = (mi_int)strlen(s);
    char* data = (char*)malloc((size_t)(len + 1));
    memcpy(data, s, (size_t)(len + 1));
    return (mi_str){ data, len, len };
}

mi_str mi_str_from(char* data, mi_int len, mi_int cap) {
    return (mi_str){ data, len, cap };
}

void mi_str_free(mi_str* s) {
    if (s->data) { free(s->data); s->data = NULL; }
    s->len = 0; s->cap = 0;
}

mi_str mi_str_clone(mi_str s) {
    char* data = (char*)malloc((size_t)s.cap + 1);
    memcpy(data, s.data, (size_t)s.len);
    data[s.len] = '\0';
    return (mi_str){ data, s.len, s.cap };
}

mi_int mi_str_len(mi_str s) { return s.len; }

mi_bool mi_str_eq(mi_str a, mi_str b) {
    if (a.len != b.len) return 0;
    return memcmp(a.data, b.data, (size_t)a.len) == 0;
}

mi_bool mi_str_ne(mi_str a, mi_str b) { return !mi_str_eq(a, b); }
mi_bool mi_str_lt(mi_str a, mi_str b) { return strcmp(a.data, b.data) < 0; }
mi_bool mi_str_le(mi_str a, mi_str b) { return strcmp(a.data, b.data) <= 0; }
mi_bool mi_str_gt(mi_str a, mi_str b) { return strcmp(a.data, b.data) > 0; }
mi_bool mi_str_ge(mi_str a, mi_str b) { return strcmp(a.data, b.data) >= 0; }

mi_str mi_str_concat(mi_str a, mi_str b) {
    mi_int new_len = a.len + b.len;
    char* data = (char*)malloc((size_t)new_len + 1);
    memcpy(data, a.data, (size_t)a.len);
    memcpy(data + a.len, b.data, (size_t)b.len);
    data[new_len] = '\0';
    return (mi_str){ data, new_len, new_len };
}

mi_char mi_str_at(mi_str s, mi_int i) {
    if (i < 0 || i >= s.len) return '\0';
    return s.data[i];
}

/* === Array Implementation === */

mi_array mi_array_new(mi_int elem_size, mi_int cap) {
    void* data = malloc((size_t)(elem_size * cap));
    return (mi_array){ data, 0, cap, elem_size };
}

void mi_array_free(mi_array* arr) {
    if (arr->data) { free(arr->data); arr->data = NULL; }
    arr->len = 0; arr->cap = 0;
}

mi_int mi_array_len(mi_array* arr) { return arr->len; }

void* mi_array_get_ptr(mi_array* arr, mi_int i) {
    return (char*)arr->data + (i * arr->elem_size);
}

mi_int mi_array_get_int(mi_array* arr, mi_int i) {
    mi_int val;
    memcpy(&val, (char*)arr->data + (i * arr->elem_size), sizeof(mi_int));
    return val;
}

void mi_array_set(mi_array* arr, mi_int i, void* val) {
    memcpy((char*)arr->data + (i * arr->elem_size), val, arr->elem_size);
}

void mi_array_push(mi_array* arr, void* val) {
    if (arr->len >= arr->cap) {
        arr->cap = arr->cap ? arr->cap * 2 : 8;
        arr->data = realloc(arr->data, (size_t)(arr->elem_size * arr->cap));
    }
    mi_array_set(arr, arr->len, val);
    arr->len++;
}

mi_array mi_array_from_lit(void* data, mi_int len, mi_int elem_size) {
    void* new_data = malloc((size_t)(elem_size * len));
    memcpy(new_data, data, (size_t)(elem_size * len));
    return (mi_array){ new_data, len, len, elem_size };
}

mi_str mi_array_to_string(mi_array* arr, mi_str (*elem_to_str)(void*)) {
    // ponytail: basic array-to-string, can be optimized
    mi_str_builder sb = mi_sb_new();
    mi_sb_append_cstr(&sb, "[");
    for (mi_int i = 0; i < arr->len; i++) {
        if (i > 0) mi_sb_append_cstr(&sb, ", ");
        mi_str elem = elem_to_str((char*)arr->data + i * arr->elem_size);
        mi_sb_append(&sb, elem);
    }
    mi_sb_append_cstr(&sb, "]");
    return mi_sb_build(&sb);
}

/* === Built-in Output Functions === */

void mi_print_int(mi_int n)    { printf("%lld", (long long)n); }
void mi_print_float(mi_float f){ printf("%g", f); }
void mi_print_bool(mi_bool b)  { printf("%s", b ? "true" : "false"); }
void mi_print_str(mi_str s)    { printf("%.*s", (int)s.len, s.data); }
void mi_println_str(mi_str s)  { printf("%.*s\n", (int)s.len, s.data); }
void mi_print(mi_str s)        { mi_print_str(s); }
void mi_println(mi_str s)      { mi_println_str(s); }

/* === Conversion Helpers === */

mi_str mi_to_string(mi_int n) {
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)n);
    return mi_str_lit(buf);
}

mi_str mi_float_to_string(mi_float f) {
    char buf[64];
    int len = snprintf(buf, sizeof(buf), "%g", f);
    return mi_str_lit(buf);
}

mi_str mi_bool_to_string(mi_bool b) {
    return mi_str_lit(b ? "true" : "false");
}

mi_int mi_from_string(mi_str s) {
    return (mi_int)atoll(s.data);
}

/* === Input === */

mi_str mi_read_line(void) {
    char buf[4096];
    if (!fgets(buf, sizeof(buf), stdin)) return mi_str_empty();
    mi_int len = (mi_int)strlen(buf);
    if (len > 0 && buf[len-1] == '\n') buf[--len] = '\0';
    return mi_str_from(strdup(buf), len, len);
}

/* === Comparison === */

mi_bool mi_int_eq(mi_int a, mi_int b)    { return a == b; }
mi_bool mi_float_eq(mi_float a, mi_float b){ return a == b; }

/* === MATH LIBRARY === */

mi_float mi_pi(void)     { return 3.14159265358979323846; }
mi_float mi_e(void)      { return 2.71828182845904523536; }
mi_float mi_sin(mi_float x)   { return sin(x); }
mi_float mi_cos(mi_float x)   { return cos(x); }
mi_float mi_tan(mi_float x)   { return tan(x); }
mi_float mi_sqrt(mi_float x)  { return sqrt(x); }
mi_float mi_pow(mi_float x, mi_float y) { return pow(x, y); }
mi_float mi_exp(mi_float x)   { return exp(x); }
mi_float mi_log(mi_float x)   { return log(x); }
mi_float mi_log10(mi_float x) { return log10(x); }
mi_float mi_abs(mi_float x)   { return fabs(x); }
mi_float mi_floor(mi_float x) { return floor(x); }
mi_float mi_ceil(mi_float x)  { return ceil(x); }
mi_float mi_round(mi_float x) { return round(x); }
mi_float mi_atan2(mi_float y, mi_float x) { return atan2(y, x); }

/* === FILE I/O === */

mi_str mi_read_file(mi_str path) {
    FILE* f = fopen(path.data, "rb");
    if (!f) return mi_str_empty();
    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    rewind(f);
    char* data = (char*)malloc((size_t)len + 1);
    size_t nread = fread(data, 1, (size_t)len, f);
    fclose(f);
    data[nread] = '\0';
    return mi_str_from(data, (mi_int)nread, (mi_int)nread);
}

mi_bool mi_write_file(mi_str path, mi_str content) {
    FILE* f = fopen(path.data, "wb");
    if (!f) return 0;
    size_t written = fwrite(content.data, 1, (size_t)content.len, f);
    fclose(f);
    return (mi_bool)(written == (size_t)content.len);
}

mi_bool mi_file_exists(mi_str path) {
    FILE* f = fopen(path.data, "r");
    if (f) { fclose(f); return 1; }
    return 0;
}

/* === MEMORY ARENA === */

struct mi_arena {
    void** blocks;
    mi_int count;
    mi_int cap;
};

mi_arena* mi_arena_new(void) {
    mi_arena* a = (mi_arena*)malloc(sizeof(mi_arena));
    a->blocks = (void**)malloc(sizeof(void*) * 64);
    a->count = 0;
    a->cap = 64;
    return a;
}

void* mi_arena_alloc(mi_arena* a, mi_int size) {
    if (a->count >= a->cap) {
        a->cap *= 2;
        a->blocks = (void**)realloc(a->blocks, sizeof(void*) * (size_t)a->cap);
    }
    void* ptr = malloc((size_t)size);
    a->blocks[a->count++] = ptr;
    return ptr;
}

void mi_arena_free_all(mi_arena* a) {
    for (mi_int i = 0; i < a->count; i++) {
        free(a->blocks[i]);
    }
    a->count = 0;
}

void mi_arena_free(mi_arena* a) {
    mi_arena_free_all(a);
    free(a->blocks);
    free(a);
}

/* === STRING BUILDER === */

mi_str_builder mi_sb_new(void) {
    mi_str_builder sb;
    sb.buf = mi_str_empty();
    return sb;
}

void mi_sb_append(mi_str_builder* sb, mi_str s) {
    mi_str old = sb->buf;
    sb->buf = mi_str_concat(old, s);
    free(old.data);
}

void mi_sb_append_cstr(mi_str_builder* sb, const char* s) {
    mi_str tmp = mi_str_lit(s);
    mi_sb_append(sb, tmp);
    free(tmp.data);
}

void mi_sb_append_int(mi_str_builder* sb, mi_int n) {
    mi_str tmp = mi_to_string(n);
    mi_sb_append(sb, tmp);
    free(tmp.data);
}

void mi_sb_append_float(mi_str_builder* sb, mi_float f) {
    mi_str tmp = mi_float_to_string(f);
    mi_sb_append(sb, tmp);
    free(tmp.data);
}

mi_str mi_sb_build(mi_str_builder* sb) {
    mi_str result = mi_str_clone(sb->buf);
    return result;
}

void mi_sb_free(mi_str_builder* sb) {
    free(sb->buf.data);
    sb->buf.data = NULL;
    sb->buf.len = 0;
    sb->buf.cap = 0;
}

/* === CLI Args === */
static char** _mi_saved_argv = NULL;
static mi_int _mi_saved_argc = 0;

void mi_init_args(int argc, char** argv) {
    _mi_saved_argc = argc;
    _mi_saved_argv = argv;
}

mi_array os_args(void) {
    mi_array args = mi_array_new(sizeof(mi_str), _mi_saved_argc);
    for (mi_int i = 0; i < _mi_saved_argc; i++) {
        mi_str s = mi_str_lit(_mi_saved_argv[i]);
        mi_array_set(&args, i, &s);
    }
    args.len = _mi_saved_argc;
    return args;
}
