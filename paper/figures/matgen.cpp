#include "common.hpp"
#include "estimate.hpp"
#include "names.hpp"
int main(int argc,char**argv){
    assert(argc==3); int n=atoi(argv[1]); uint64_t seed=atoll(argv[2]);
    mt19937_64 gen(seed); vector<Edge> e;
    for(int x=0;x<n;x++)for(int y=x+1;y<n;y++){W w=1+(gen()&15);if(gen()&1)w+=W(1)<<40;e.push_back({x,y,w});}
    mt19937_64 rng(1234567+seed);
    auto names=name_routes(n,e,rng);
    size_t removed=0;
    printf("n %d edges %zu P %llu\n",n,e.size(),(unsigned long long)names.P);
    for(size_t i=0;i<e.size();++i){
        // Same certificate test as apply(): deletable iff 2*P*lambda admits a strictly cheaper detour.
        bool gone=(unsigned __int128)2*names.P*names.lambda[i]<=e[i].w;
        removed+=gone;
        printf("%d %d %d %d\n",e[i].x,e[i].y,e[i].w>=W(1)<<40?2:1,(int)gone);
    }
    fprintf(stderr,"removed %zu\n",removed);
}
