/* Linux/glibc test fixture: observe the actual dynamically loaded Rust allocator.
 * Thread-local counters and direct glibc entrypoints avoid recursion or allocation.
 */
#include <stddef.h>
#include <stdint.h>
#include <errno.h>
extern void *__libc_malloc(size_t);
extern void *__libc_calloc(size_t,size_t);
extern void *__libc_realloc(void*,size_t);
extern void *__libc_memalign(size_t,size_t);
extern void __libc_free(void*);
static _Thread_local int armed;
static _Thread_local uint64_t allocations, deallocations;
void dandrum_audit_begin(void) { allocations=0; deallocations=0; armed=1; }
void dandrum_audit_end(uint64_t *a,uint64_t *d) { armed=0; *a=allocations; *d=deallocations; }
void *malloc(size_t n) { void *p=__libc_malloc(n); if(armed && p) ++allocations; return p; }
void *calloc(size_t n,size_t s) { void *p=__libc_calloc(n,s); if(armed && p) ++allocations; return p; }
void *realloc(void *p,size_t n) { void *q=__libc_realloc(p,n); if(armed) { if(q) ++allocations; if(p && (q || !n)) ++deallocations; } return q; }
void free(void *p) { if(armed && p) ++deallocations; __libc_free(p); }
int posix_memalign(void **p,size_t alignment,size_t n) {
 if(alignment < sizeof(void*) || alignment % sizeof(void*) || (alignment & (alignment-1))) return EINVAL;
 void *q=__libc_memalign(alignment,n); if(!q) return ENOMEM;
 *p=q; if(armed) ++allocations; return 0;
}
void *aligned_alloc(size_t alignment,size_t n) {void *p=__libc_memalign(alignment,n);if(armed && p)++allocations;return p;}
void *memalign(size_t alignment,size_t n) {void *p=__libc_memalign(alignment,n);if(armed && p)++allocations;return p;}
