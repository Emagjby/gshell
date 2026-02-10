#include <unistd.h>
#include <string.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

#include "fs.h"
#include "argvec.h"
#include "helpers.h"
#include "commands.h"
#include "error.h"
#include "dynbuf.h"

void type_command(ArgVec argv) {
  if (argv.count < 2) {
    error(ERROR_INSUFFICIENT_ARGUMENTS, "type");
  }

  if(is_builtin_command(argv.args[1])) {
    builtin_type(argv.args[1]);
    return;
  }

  char* found = check_path_directories(argv.args[1]);
  if (found) {
    // build full path
    char* full_path = build_full_path(found, argv.args[1]);

    DynBuf dynbuf;
    dynbuf_init(&dynbuf);

    dynbuf_append(&dynbuf, argv.args[1]);
    dynbuf_append(&dynbuf, " is ");
    dynbuf_append(&dynbuf, full_path);
    dynbuf_append(&dynbuf, "\n");

    write(STDOUT_FILENO, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
    free(full_path);
  } else {
    unknown_type(argv.args[1]);
  }
}

void clear_command() {
  write(STDOUT_FILENO, "\x1b[H\x1b[2J", 7);
}

void echo_command(ArgVec argv) {
  for(size_t i = 1; i < argv.count; i++) {
    write(STDOUT_FILENO, argv.args[i], strlen(argv.args[i]));
    if (i < argv.count - 1) {
      write(STDOUT_FILENO, " ", 1);
    }
  }

  write(STDOUT_FILENO, "\n", 1);
}

void run_command(ArgVec argv, char* path) {
  char* full_path = build_full_path(path, argv.args[0]);

  run_program(full_path, argv.args);

  free(full_path);
}

void pwd_command(void) {
  char cwd[PATH_MAX];
  getcwd(cwd, sizeof(cwd));
  write(STDOUT_FILENO, cwd, strlen(cwd));
  write(STDOUT_FILENO, "\n", 1);
}

void cd_command(ArgVec argv) {
  if (argv.count < 2) {
    char* home = getenv("HOME");
    if (home == NULL) {
      error(ERROR_ENVIRONMENT_VARIABLE_NOT_SET, "HOME");
    }
    chdir(home);
    return;
  }

  char* path = strcpy(malloc(strlen(argv.args[1]) + 1), argv.args[1]);
  handle_home(&path);

  if (chdir(path) != 0) {
    error(ERROR_CD_NO_SUCH_DIRECTORY, path);
  }
  free(path);
}
