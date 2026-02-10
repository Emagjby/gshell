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
#include "linenoise.h"

static size_t history_last_append_index = 0;

static size_t history_len_from_list(char** history) {
  size_t count = 0;
  if(!history) {
    return 0;
  }

  while(history[count]) {
    count++;
  }
  return count;
}

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

static void append_history(DynBuf* dynbuf) {
  char** history = get_history();
  if(!history) {
    return;
  }

  for(size_t i = 0; history[i]; i++) {
    char num[64];
    snprintf(num, sizeof(num), "%4zu ", i + 1);
    dynbuf_append(dynbuf, num);
    dynbuf_append(dynbuf, history[i]);
    dynbuf_append(dynbuf, "\n");
  }
}

static void append_last_n_history(DynBuf* dynbuf, size_t n) {
  char** history = get_history();
  if(!history) {
    return;
  }

  size_t start = 0;
  for(size_t i = 0; history[i]; i++) {
    start = i + 1;
  }

  if (n > start) {
    n = start;
  }

  for(size_t i = start - n; i < start; i++) {
    char num[64];
    snprintf(num, sizeof(num), "%4zu ", i + 1);
    dynbuf_append(dynbuf, num);
    dynbuf_append(dynbuf, history[i]);
    dynbuf_append(dynbuf, "\n");
  }
}

static void history_read_command(ArgVec argv) {
  if (argv.count < 3) {
    error(ERROR_INSUFFICIENT_ARGUMENTS, "history -r");
  }

  char* filename = argv.args[2];
  if (linenoiseHistoryLoad(filename) != 0) {
    error(ERROR_HISTORY_LOAD, filename);
  }

  history_last_append_index = history_len_from_list(get_history());
}

static void history_write_command(ArgVec argv) {
  if (argv.count < 3) {
    error(ERROR_INSUFFICIENT_ARGUMENTS, "history -w");
  }

  char* filename = argv.args[2];
  if (linenoiseHistorySave(filename) != 0) {
    error(ERROR_FILE_OPERATION_FAILED, filename);
  }
}

static void history_append_command(ArgVec argv) {
  if (argv.count < 3) {
    error(ERROR_INSUFFICIENT_ARGUMENTS, "history -a");
  }

  char* filename = argv.args[2];
  DynBuf dynbuf;
  dynbuf_init(&dynbuf);

  // get history and append to dynbuf 
  char** history = get_history();
  size_t total = history_len_from_list(history);
  if (history_last_append_index > total) {
    history_last_append_index = 0;
  }

  for(size_t i = history_last_append_index; i < total; i++) {
    dynbuf_append(&dynbuf, history[i]);
    dynbuf_append(&dynbuf, "\n");
  }

  if (dynbuf.len == 0) {
    dynbuf_free(&dynbuf);
    history_last_append_index = total;
    return;
  }

  FILE*file = fopen(filename, "a");
  if (!file) {
    error(ERROR_FILE_OPERATION_FAILED, filename);
  }

  fprintf(file, "%s", dynbuf.buf);
  fclose(file);
  dynbuf_free(&dynbuf);
  history_last_append_index = total;
}

void history_command(ArgVec argv) {
  DynBuf dynbuf;
  dynbuf_init(&dynbuf);

  if(argv.count > 1) {
    if(strcmp(argv.args[1], "-a") == 0) {
      history_append_command(argv);
    } else if(strcmp(argv.args[1], "-r") == 0) {
      history_read_command(argv);
    } else if(strcmp(argv.args[1], "-w") == 0) {
      history_write_command(argv);
    } else if(is_number(argv.args[1])) {
      // turn the arg to a num
      char* endptr;
      long num = strtol(argv.args[1], &endptr, 10);
      if (*endptr != '\0' || num <= 0) {
        dynbuf_free(&dynbuf);
        error(ERROR_INVALID_ARGUMENT, argv.args[1]);
      }

      append_last_n_history(&dynbuf, (size_t)num);
    } else {
      dynbuf_free(&dynbuf);
      error(ERROR_INVALID_ARGUMENT, argv.args[1]);
    }
  } else {
    append_history(&dynbuf);
  }

  write(STDOUT_FILENO, dynbuf.buf, dynbuf.len);
  dynbuf_free(&dynbuf);
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
