#include <string.h>
#include <stdio.h>
#include <unistd.h>
#include <limits.h>
#include <stdlib.h>
#include <sys/wait.h>

#include "argvec.h"
#include "command.h"
#include "pipeline.h"
#include "error.h"
#include "fs.h"
#include "commands.h"
#include "helpers.h"

extern char** environ;

void execute_builtin(Command* command) {
  ArgVec argv = command->argv;
  if(argv.count == 0 || argv.args == NULL || argv.args[0] == NULL) {
    return;
  }
  char* toExec = argv.args[0];

  if(strcmp(toExec, "exit") == 0) {
    exit(0);
  } else if (strcmp(toExec, "echo") == 0) {
    echo_command(argv);
  } else if (strcmp(toExec, "clear") == 0) {
    clear_command();
    return;
  } else if (strcmp(toExec, "type") == 0) {
    type_command(argv);
  } else if (strcmp(toExec, "pwd") == 0) {
    pwd_command();
  } else if (strcmp(toExec, "cd") == 0) {
    cd_command(argv);
  } 
}

void execute(Command* command) {
  ArgVec argv = command->argv;
  if(argv.count == 0 || argv.args == NULL || argv.args[0] == NULL) {
    return;
  }
  char* toExec = argv.args[0]; 

  if(is_builtin_command(toExec)) {
    execute_builtin(command);
    return;
  } else {
    char* found_path = check_path_directories(toExec);

    if (!found_path) {
      error(ERROR_COMMAND_NOT_FOUND, toExec);
      free(found_path);
      return;
    }

    run_command(command->argv, found_path);
    free(found_path);
  }
}


void execute_pipeline(Pipeline* pipeline) {
  if(pipeline->count == 0) {
    return;
  }

  // no right command, execute left only
  if (pipeline->count == 1) {
    Command* command = pipeline->commands[0];
    int discarded_stdout = -1;
    int discarded_stderr = -1;

    if(command->stdout_path) {
      discarded_stdout = redirect_stdout(command->stdout_path, REDIRECT_OUT);
      if(discarded_stdout != -1) {
        close(discarded_stdout);
      }
    }

    if(command->stderr_path) {
      discarded_stderr = redirect_stderr(command->stderr_path, REDIRECT_OUT);
      if(discarded_stderr != -1) {
        close(discarded_stderr);
      }
    }

    if(command->stdout_append) {
      discarded_stdout = redirect_stdout(command->stdout_append, REDIRECT_APPEND);
      if(discarded_stdout != -1) {
        close(discarded_stdout);
      }
    }

    if(command->stderr_append) {
      discarded_stderr = redirect_stderr(command->stderr_append, REDIRECT_APPEND);
      if(discarded_stderr != -1) {
        close(discarded_stderr);
      }
    }

    execute(command);

    return;
  }

  for(size_t i = 0; i < pipeline->count; i++) {
    Command* command = pipeline->commands[i];
    if(command->argv.count == 0 || command->argv.args == NULL || command->argv.args[0] == NULL) {
      continue;
    }
    if(is_builtin_command(command->argv.args[0])) {
      continue;
    }
    char* path = check_path_directories(command->argv.args[0]);
    if(!path) {
      error(ERROR_COMMAND_NOT_FOUND, command->argv.args[0]);
      return;
    }
    free(path);
  }


  // WARNING: can cause stack overflow if pipeline is too long, but should be fine for typical use cases
  // deliberately using VLA for simplicity, but could be changed to dynamic allocation if needed
  int pipes[pipeline->count - 1][2];

  for(size_t i = 0; i < pipeline->count - 1; i++) {
    if (pipe(pipes[i]) == -1) {
      for(size_t j = 0; j < i; j++) {
        close(pipes[j][0]);
        close(pipes[j][1]);
      }

      error(ERROR_EXECUTE_ERROR, "Failed to create pipe");
    }
  }

  size_t spawned = 0;

  for(size_t i = 0; i < pipeline->count; i++) {
    pid_t pid = fork();
    if (pid == -1) {
      for(size_t j = 0; j < pipeline->count - 1; j++) {
        close(pipes[j][0]);
        close(pipes[j][1]);
      }

      for(size_t j = 0; j < spawned; j++) {
        // TODO: track child PIDs and propagate last command exit status
        wait(NULL);
      }

      error(ERROR_EXECUTE_ERROR, "Failed to fork process");
    } else if (pid == 0) {
      // child process
      if (i > 0) {
        // not first command, redirect stdin to read end of previous pipe
        dup2(pipes[i - 1][0], STDIN_FILENO);
      }
      if (i < pipeline->count - 1) {
        // not last command, redirect stdout to write end of current pipe
        dup2(pipes[i][1], STDOUT_FILENO);
      }

      // close all pipe fds in child
      for(size_t j = 0; j < pipeline->count - 1; j++) {
        close(pipes[j][0]);
        close(pipes[j][1]);
      }

      Command* command = pipeline->commands[i];

      if(command->stdout_path) {
        redirect_stdout(command->stdout_path, REDIRECT_OUT);
      }

      if(command->stderr_path) {
        redirect_stderr(command->stderr_path, REDIRECT_OUT);
      }

      if(command->stdout_append) {
        redirect_stdout(command->stdout_append, REDIRECT_APPEND);
      }

      if(command->stderr_append) {
        redirect_stderr(command->stderr_append, REDIRECT_APPEND);
      }

      if(command->argv.args == NULL || command->argv.count == 0 || command->argv.args[0] == NULL) {
        _exit(0);
      }

      if(is_builtin_command(command->argv.args[0])) {
        execute_builtin(command);
        _exit(0);
      } else {
        char* path = check_path_directories(command->argv.args[0]);
        if(!path) {
          error_no_panic(ERROR_COMMAND_NOT_FOUND, command->argv.args[0]);
          _exit(127);
        }
        char* full_path = build_full_path(path, command->argv.args[0]);
        free(path);

        if(!full_path) {
          _exit(127);
        }
        execve(full_path, command->argv.args, environ);
        perror("execve");
        _exit(1);
      }

      exit(0);
    }

    spawned++;
  }

  for(size_t i = 0; i < pipeline->count - 1; i++) {
    // close all pipe fds in parent
    close(pipes[i][0]);
    close(pipes[i][1]);
  }

  for(size_t i = 0; i < pipeline->count; i++) {
    // TODO: track child PIDs and propagate last command exit status
    wait(NULL);
  }
}
