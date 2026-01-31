#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
  setbuf(stdout, NULL);

  for(;;) {
    // Display prompt
    char prompt[2] = {'$', ' '};
    write(1, prompt, sizeof(prompt));

    // Read user input & remove \n
    char *command = NULL;
    size_t cap = 0;

    ssize_t read = getline(&command, &cap, stdin);
    if (read > 0 && command[read - 1] == '\n') {
      command[read - 1] = '\0';
      read--;
    }
    
    // Error handling
    char buf[128];
    int len = snprintf(buf, sizeof(buf), "%s: command not found", command);
    write(1, buf, len);

    // Print newline before exiting
    write(1, "\n", 1);


    // Free allocated memory
    free(command);
  }

  return 0;
}
