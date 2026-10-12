#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <sys/stat.h>
#include <unistd.h>
int main(int argc, char **argv) {
 assert(argc == 2);
 int fd=open(argv[1],O_CREAT|O_RDWR|O_TRUNC,0600);assert(fd>=0);assert(ftruncate(fd,4096)==0);
 int ro=open(argv[1],O_RDONLY);assert(ro>=0);
 int server=socket(AF_INET,SOCK_STREAM,0);assert(server>=0);
 struct sockaddr_in a={.sin_family=AF_INET,.sin_addr.s_addr=htonl(INADDR_LOOPBACK),.sin_port=0};
 assert(bind(server,(void*)&a,sizeof a)==0);assert(listen(server,1)==0);
 socklen_t n=sizeof a;assert(getsockname(server,(void*)&a,&n)==0);
 fprintf(stderr,"E2_QUERY_BEGIN control-fd\n");
 assert(pwrite(fd,"X",1,0)==1);assert(pwrite(ro,"Y",1,1)==-1 && errno==EBADF);
 assert(ftruncate(fd,4096)==0);assert(fchmod(fd,0600)==0);
 fprintf(stderr,"E2_QUERY_END control-fd\n");
 fprintf(stderr,"E2_QUERY_BEGIN control-map\n");
 assert(mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_SHARED,ro,0)==MAP_FAILED);
 void *m=mmap(NULL,4096,PROT_READ,MAP_SHARED,fd,0);assert(m!=MAP_FAILED);
 assert(mprotect(m,4096,PROT_READ|PROT_WRITE)==0);((char*)m)[0]='Z';assert(msync(m,4096,MS_SYNC)==0);assert(munmap(m,4096)==0);
 void *p=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE,fd,0);assert(p!=MAP_FAILED);assert(munmap(p,4096)==0);
 void *anon=mmap(NULL,4096,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS,-1,0);assert(anon!=MAP_FAILED);assert(munmap(anon,4096)==0);
 fprintf(stderr,"E2_QUERY_END control-map\n");
 fprintf(stderr,"E2_QUERY_BEGIN control-network\n");
 int client=socket(AF_INET,SOCK_STREAM,0);assert(client>=0);assert(connect(client,(void*)&a,sizeof a)==0);assert(write(client,"N",1)==1);
 fprintf(stderr,"E2_QUERY_END control-network\n");
 close(client);close(server);close(ro);close(fd);puts("owned fd/mapping/loopback controls executed");return 0;
}
