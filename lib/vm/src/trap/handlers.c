// This file contains partial code from other sources.
// Attributions: https://github.com/wasmerio/wasmer/blob/master/ATTRIBUTIONS.md

#include <setjmp.h>
#include <stdio.h>
#if defined(CFG_TARGET_OS_MACOS)
#include <mach/task.h>
#include <mach/mach_init.h>
#include <mach/mach_port.h>
#endif
// Note that `sigsetjmp` and `siglongjmp` are used here where possible to
// explicitly pass a 0 argument to `sigsetjmp` that we don't need to preserve
// the process signal mask. This should make this call a bit faster b/c it
// doesn't need to touch the kernel signal handling routines.
// In case of macOS, stackoverflow
#if defined(CFG_TARGET_OS_WINDOWS)
#define platform_setjmp(buf) setjmp(buf)
#define platform_longjmp(buf, arg) longjmp(buf, arg)
#define platform_jmp_buf jmp_buf
#elif defined(CFG_TARGET_OS_MACOS)
#define platform_setjmp(buf) sigsetjmp(buf, 1)
#define platform_longjmp(buf, arg) siglongjmp(buf, arg)
#define platform_jmp_buf sigjmp_buf
#else
#define platform_setjmp(buf) sigsetjmp(buf, 0)
#define platform_longjmp(buf, arg) siglongjmp(buf, arg)
#define platform_jmp_buf sigjmp_buf
#endif

int wasmer_register_setjmp(
    void **buf_storage,
    void (*body)(void*),
    void *payload) {
  #if 0 && defined(CFG_TARGET_OS_MACOS)
  // Enable this block to ba able to debug Segfault with lldb
  // This will mask the EXC_BAD_ACCESS from lldb...
  static int allow_bad_access = 0;
  if(!allow_bad_access) {
    allow_bad_access = 1;
    task_set_exception_ports(mach_task_self(), EXC_MASK_BAD_ACCESS, MACH_PORT_NULL, EXCEPTION_DEFAULT, 0);
  }
  #endif
  platform_jmp_buf buf;
  if (platform_setjmp(buf) != 0) {
    return 0;
  }
  *buf_storage = &buf;
  body(payload);
  return 1;
}

void wasmer_unwind(void *JmpBuf) {
  platform_jmp_buf *buf = (platform_jmp_buf*) JmpBuf;
  platform_longjmp(*buf, 1);
}
