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
    free_argvec(&argv);
    error(ERROR_INSUFFICIENT_ARGUMENTS, "type");
    return;
  }

  if(is_builtin_command(argv.args[1])) {
    builtin_type(argv.args[1]);
    free_argvec(&argv);
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

    write(1, dynbuf.buf, dynbuf.len);
    dynbuf_free(&dynbuf);
    free_argvec(&argv);
    free(full_path);
  } else {
    unknown_type(argv.args[1]);
    free_argvec(&argv);
  }
}

void clear_command() {
  write(1, "\x1b[H\x1b[2J", 7);
}

void echo_command(ArgVec argv) {
  for(int i = 1; i < argv.count; i++) {
    write(1, argv.args[i], strlen(argv.args[i]));
    if (i < argv.count - 1) {
      write(1, " ", 1);
    }
  }

  write(1, "\n", 1);
  free_argvec(&argv);
}

void run_command(ArgVec argv, char* path) {
  char* full_path = build_full_path(path, argv.args[0]);

  run_program(full_path, argv.args);

  free(full_path);
  free_argvec(&argv);
}

void pwd_command(void) {
  char cwd[PATH_MAX];
  getcwd(cwd, sizeof(cwd));
  write(1, cwd, strlen(cwd));
  write(1, "\n", 1);
}

void cd_command(ArgVec argv) {
  if (argv.count < 2) {
    char* home = getenv("HOME");
    if (home == NULL) {
      free_argvec(&argv);
      error(ERROR_ENVIRONMENT_VARIABLE_NOT_SET, "HOME");
      return;
    }
    chdir(getenv("HOME"));
    return;
  }

  char* path = strcpy(malloc(strlen(argv.args[1]) + 1), argv.args[1]);
  handle_home(&path, &argv);

  if (chdir(path) != 0) {
    free_argvec(&argv);
    error(ERROR_CD_NO_SUCH_DIRECTORY, path);
  }
  free_argvec(&argv);
}
