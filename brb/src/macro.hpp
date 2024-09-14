#ifndef MACRO_H
#define MACRO_H
#include <stdarg.h>
#include <stdio.h>
#define PRINT_INFO
// #define PRINT_DEBUG
#define PRINT_RED "\033[31m"
#define PRINT_GREEN "\033[32m"
#define PRINT_YELLOW "\033[33m"
#define PRINT_NORMAL "\033[0m"

void
print_debug(const char *fmt, ...)
{
    #ifdef PRINT_DEBUG
    va_list args;
    va_start(args, fmt);
    printf(PRINT_GREEN"\tBCB DEBUG: ");
    vprintf(fmt, args);
    va_end(args);
    printf(PRINT_NORMAL"\n");
    #endif
}

void 
print_info(const char *fmt, ...)
{
    #if defined(PRINT_INFO) || defined(PRINT_DEBUG)
    va_list args;
    va_start(args, fmt);
    printf(PRINT_YELLOW"BCB INFO: ");
    vprintf(fmt, args);
    va_end(args);
    printf(PRINT_NORMAL"\n");
    #endif
}

void
print_err(const char *fmt, ...)
{
    va_list args;
    va_start(args, fmt);
    printf(PRINT_RED"ERROR: ");
    vprintf(fmt, args);
    va_end(args);
    printf(PRINT_NORMAL"\n");
}
#endif 