#ifndef HISTORY_H_
#define HISTORY_H_

#define HISTORY_SIZE 4096

void history_init(void);
void history_add(const char* line);

#endif
