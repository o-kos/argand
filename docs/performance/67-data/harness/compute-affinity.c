#define _GNU_SOURCE
#include <pthread.h>
#include <sched.h>
#include <stdlib.h>
#include <string.h>
#include <dlfcn.h>
#include <stdio.h>
int pthread_setname_np(pthread_t thread, const char *name) {
    int (*real_fn)(pthread_t,const char*) = dlsym(RTLD_NEXT,"pthread_setname_np");
    int result = real_fn(thread,name);
    const char *mask = getenv("BENCH_COMPUTE_CPUS");
    if (mask && strncmp(name,"argand-fft-",11)==0) {
        cpu_set_t cpus; CPU_ZERO(&cpus);
        char *copy=strdup(mask), *save=NULL;
        for(char *p=strtok_r(copy,",",&save);p;p=strtok_r(NULL,",",&save)) {
            int cpu=atoi(p); if(cpu>=0 && cpu<CPU_SETSIZE) CPU_SET(cpu,&cpus);
        }
        int error=pthread_setaffinity_np(thread,sizeof(cpus),&cpus);
        if(error) fprintf(stderr,"BENCH affinity error: %d\n",error);
        free(copy);
    }
    return result;
}
