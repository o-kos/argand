#define _GNU_SOURCE
#include <wayland-client.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "screencopy.h"
#include "presentation.h"
#include "pointer.h"
static struct wl_display *display;
static struct wl_shm *shm;
static struct wl_output *output;
static struct zwlr_screencopy_manager_v1 *manager;
static struct zwlr_virtual_pointer_manager_v1 *pmanager;
static struct zwlr_virtual_pointer_v1 *pointer;
static struct wl_buffer *buffer;
static unsigned char *pixels;
static size_t allocation;
static uint32_t stride,height,flags;
static int click_x,click_y,probe_x,probe_y,frames,done;
static uint64_t baseline,started;
static clockid_t presentation_clock=CLOCK_MONOTONIC;
static int received_clock;
static void clock_event(void *d,struct wp_presentation *p,uint32_t id){(void)d;(void)p;presentation_clock=id;received_clock=1;}
static const struct wp_presentation_listener p_listener={.clock_id=clock_event};
static uint64_t now(void){struct timespec t;clock_gettime(presentation_clock,&t);return (uint64_t)t.tv_sec*1000000000+t.tv_nsec;}
static void capture(void);
static void global(void *d,struct wl_registry *r,uint32_t n,const char *i,uint32_t v){
 (void)d;(void)v;
 if(!strcmp(i,"wp_presentation")){struct wp_presentation *p=wl_registry_bind(r,n,&wp_presentation_interface,1);wp_presentation_add_listener(p,&p_listener,NULL);}
 if(!strcmp(i,"wl_shm"))shm=wl_registry_bind(r,n,&wl_shm_interface,1);
 if(!strcmp(i,"wl_output")&&!output)output=wl_registry_bind(r,n,&wl_output_interface,1);
 if(!strcmp(i,"zwlr_screencopy_manager_v1"))manager=wl_registry_bind(r,n,&zwlr_screencopy_manager_v1_interface,1);
 if(!strcmp(i,"zwlr_virtual_pointer_manager_v1"))pmanager=wl_registry_bind(r,n,&zwlr_virtual_pointer_manager_v1_interface,1);
}
static void removed(void *d,struct wl_registry *r,uint32_t n){(void)d;(void)r;(void)n;}
static void got_buffer(void *d,struct zwlr_screencopy_frame_v1 *f,uint32_t format,uint32_t w,uint32_t h,uint32_t s){
 (void)d;(void)w;stride=s;height=h;allocation=(size_t)s*h;
 int fd=memfd_create("screen-latency",0);if(fd<0||ftruncate(fd,allocation)){perror("shm");exit(2);}
 pixels=mmap(NULL,allocation,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);
 struct wl_shm_pool *pool=wl_shm_create_pool(shm,fd,allocation);
 buffer=wl_shm_pool_create_buffer(pool,0,w,h,s,format);wl_shm_pool_destroy(pool);close(fd);
 zwlr_screencopy_frame_v1_copy(f,buffer);
}
static void got_flags(void *d,struct zwlr_screencopy_frame_v1 *f,uint32_t v){(void)d;(void)f;flags=v;}
static void ready(void *d,struct zwlr_screencopy_frame_v1 *f,uint32_t hi,uint32_t lo,uint32_t ns){
 (void)d;(void)hi;(void)lo;(void)ns;uint64_t hash=1469598103934665603ULL;
 for(int y=probe_y;y<probe_y+24;y++){
  int row=(flags&1)?(int)height-1-y:y;
  for(int x=probe_x*4;x<(probe_x+24)*4;x++){hash^=pixels[(size_t)row*stride+x];hash*=1099511628211ULL;}
 }
 zwlr_screencopy_frame_v1_destroy(f);wl_buffer_destroy(buffer);munmap(pixels,allocation);
 if(frames++==0){
  baseline=hash;
  zwlr_virtual_pointer_v1_button(pointer,now()/1000000,272,1);zwlr_virtual_pointer_v1_frame(pointer);
  started=now();
  zwlr_virtual_pointer_v1_button(pointer,started/1000000,272,0);zwlr_virtual_pointer_v1_frame(pointer);
  wl_display_flush(display);
 }else if(hash!=baseline){printf("{\"click_to_changed_frame_ms\":%.3f,\"presentation_ms\":%.3f,\"frames\":%d}\n",(now()-started)/1000000.0,((((uint64_t)hi<<32)|lo)*1000000000+ns-started)/1000000.0,frames);done=1;return;}
 if(frames>100){fprintf(stderr,"no changed frame\n");exit(3);}
 capture();
}
static void failed(void *d,struct zwlr_screencopy_frame_v1 *f){(void)d;(void)f;fprintf(stderr,"capture failed\n");exit(2);}
static const struct zwlr_screencopy_frame_v1_listener listener={.buffer=got_buffer,.flags=got_flags,.ready=ready,.failed=failed};
static void capture(void){flags=0;struct zwlr_screencopy_frame_v1 *f=zwlr_screencopy_manager_v1_capture_output(manager,0,output);zwlr_screencopy_frame_v1_add_listener(f,&listener,NULL);}
int main(int argc,char **argv){
 if(argc!=5)return 2;click_x=atoi(argv[1]);click_y=atoi(argv[2]);probe_x=atoi(argv[3]);probe_y=atoi(argv[4]);
 display=wl_display_connect(NULL);if(!display)return 2;
 struct wl_registry *registry=wl_display_get_registry(display);const struct wl_registry_listener r={global,removed};wl_registry_add_listener(registry,&r,NULL);wl_display_roundtrip(display);
 if(!shm||!output||!manager||!pmanager)return 2;
 pointer=zwlr_virtual_pointer_manager_v1_create_virtual_pointer(pmanager,NULL);wl_display_roundtrip(display);
 zwlr_virtual_pointer_v1_motion_absolute(pointer,now()/1000000,click_x,click_y,1600,1000);zwlr_virtual_pointer_v1_frame(pointer);wl_display_roundtrip(display);usleep(150000);if(!received_clock)return 2;capture();
 while(!done&&wl_display_dispatch(display)>=0){}return !done;
}
