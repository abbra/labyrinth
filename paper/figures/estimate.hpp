#pragma once

struct Estimate {
    vector<int> index;
    vector<unsigned __int128> bounds;
};

namespace est_detail {
using Wide = unsigned __int128;

struct Dsu {
    vector<int> p;
    explicit Dsu(int n) : p(n,-1) {}
    int root(int x) { return p[x]<0 ? x : p[x]=root(p[x]); }
    int join_roots(int x,int y) {
        if (p[x]>p[y]) swap(x,y);
        p[x]+=p[y]; p[y]=x;
        return x;
    }
};

// Euler-tour range minima on preorder numbers; queried vertices share a tree.
struct Lca {
    vector<int> first, node;
    vector<vector<int>> rmq;
    explicit Lca(const vector<int>& parent) : first(parent.size()), rmq(1) {
        int n=parent.size();
        vector<int> head(n,-1), next(n,-1);
        for (int v=0; v<n; ++v) if (parent[v]>=0) {
            next[v]=head[parent[v]]; head[parent[v]]=v;
        }
        node.reserve(n); rmq[0].reserve(2*n);
        auto visit=[&](auto&& self,int v)->void {
            int pre=node.size(); node.push_back(v);
            first[v]=rmq[0].size(); rmq[0].push_back(pre);
            for (int c=head[v]; c>=0; c=next[c]) {
                self(self,c); rmq[0].push_back(pre);
            }
        };
        for (int v=0; v<n; ++v) if (parent[v]<0) visit(visit,v);
        int len=rmq[0].size();
        for (int step=1; 2*step<=len; step*=2) {
            vector<int> row(len-2*step+1);
            const auto& prev=rmq.back();
            for (int i=0; i<(int)row.size(); ++i) row[i]=min(prev[i],prev[i+step]);
            rmq.push_back(move(row));
        }
    }
    int query(int x,int y) const {
        int l=first[x], r=first[y]; if (l>r) swap(l,r);
        int k=bit_width(unsigned(r-l+1))-1;
        return node[min(rmq[k][l],rmq[k][r-(1<<k)+1])];
    }
};
} // namespace est_detail

inline Estimate estimate(int t,const vector<Edge>& sample,const vector<Edge>& demands,mt19937_64& rng) {
    using namespace est_detail;
    assert(t>=2 && t<=256); // (16t)^5 and finite sample distances fit in W here.
    const W T=bit_ceil(W(4*t)), T2=T*T;
    W h=1;
    for (int i=0; i<5; ++i) h*=16*t;
    const W H=bit_ceil(h);
    Estimate out{vector<int>(demands.size(),-1),{}};
    vector<Edge> edges=sample, forest;
    sort(edges.begin(),edges.end(),[](const Edge& a,const Edge& b){return a.w<b.w;});
    Dsu merged(t);
    vector<int> top(t), parent(t,-1), band;
    iota(top.begin(),top.end(),0);
    vector<W> anchors;
    for (const Edge& e:edges) {
        if (anchors.empty() || e.w>=Wide(T)*anchors.back()) anchors.push_back(e.w);
        int x=merged.root(e.x), y=merged.root(e.y);
        if (x==y) continue;
        int v=parent.size(); parent[top[x]]=parent[top[y]]=v;
        parent.push_back(-1); band.push_back(int(anchors.size())-1);
        top[merged.join_roots(x,y)]=v;
        forest.push_back(e);
    }
    vector<vector<int>> groups(anchors.size());
    {
        Lca merge_lca(parent);
        for (int i=0; i<(int)demands.size(); ++i) {
            const auto& e=demands[i];
            if (merged.root(e.x)!=merged.root(e.y)) continue;
            assert(e.x!=e.y);
            groups[band[merge_lca.query(e.x,e.y)-t]].push_back(i);
        }
    }
    Dsu contracted(t);
    size_t cut=0, lo=0, hi=0;
    for (int b=0; b<(int)anchors.size(); ++b) {
        if (groups[b].empty()) continue;
        W a=anchors[b];
        while (cut<forest.size() && Wide(T2)*forest[cut].w<a) {
            const auto& e=forest[cut++];
            contracted.join_roots(contracted.root(e.x),contracted.root(e.y));
        }
        while (lo<edges.size() && Wide(T2)*edges[lo].w<a) ++lo;
        while (hi<edges.size() && edges[hi].w<=Wide(T2)*a) ++hi;
        vector<vector<pair<int,W>>> adj;
        vector<int> id(t,-1);
        auto local=[&](int root) {
            if (id[root]<0) { id[root]=adj.size(); adj.emplace_back(); }
            return id[root];
        };
        for (size_t i=lo; i<hi; ++i) {
            const auto& e=edges[i];
            int x=contracted.root(e.x), y=contracted.root(e.y);
            if (x==y) continue;
            x=local(x); y=local(y);
            adj[x].emplace_back(y,e.w); adj[y].emplace_back(x,e.w);
        }
        const int n=adj.size(), base=out.bounds.size();
        assert(n>=2);
        vector<int> perm(n), center(n), part_parent;
        iota(perm.begin(),perm.end(),0);
        vector<W> best(n);
        using Item=pair<W,int>;
        priority_queue<Item,vector<Item>,greater<Item>> heap;
        // Linked lists of the vertices of each previous part allow linear pair grouping.
        vector<int> heads(n,-1), next(n), new_heads(n), new_next(n), seen(n), child(n);
        heads[0]=0; iota(next.begin(),next.end(),1); next.back()=-1;
        int prev_count=1, prev_start=-1;
        for (Wide delta=Wide(bit_ceil(W(t)*T2))*a; delta>=a; delta/=2) {
            shuffle(perm.begin(),perm.end(),rng);
            W j=rng()&(H-1);
            // Saturate only the comparison radius, not the returned certificate.
            W radius=numeric_limits<W>::max();
            if (delta<radius) radius=W(min<Wide>(radius,delta+((delta*j)>>countr_zero(H))));
            fill(center.begin(),center.end(),-1);
            fill(best.begin(),best.end(),numeric_limits<W>::max());
            for (int c:perm) {
                best[c]=0; heap.emplace(0,c);
                while (!heap.empty()) {
                    auto [d,x]=heap.top(); heap.pop();
                    if (d!=best[x]) continue;
                    if (center[x]<0) center[x]=c;
                    for (auto [y,w]:adj[x]) {
                        W z=d+w;
                        if (z<=radius && z<best[y]) { best[y]=z; heap.emplace(z,y); }
                    }
                }
            }
            int start=part_parent.size(), count=0;
            fill(seen.begin(),seen.end(),-1);
            for (int p=0; p<prev_count; ++p) {
                for (int x=heads[p]; x>=0; x=next[x]) {
                    int c=center[x];
                    if (seen[c]!=p) {
                        seen[c]=p; child[c]=count; new_heads[count++]=-1;
                        part_parent.push_back(prev_start<0 ? -1 : prev_start+p);
                        out.bounds.push_back(4*delta+a);
                    }
                    int q=child[c]; new_next[x]=new_heads[q]; new_heads[q]=x;
                }
            }
            heads.swap(new_heads); next.swap(new_next);
            prev_start=start; prev_count=count;
        }
        int leaves=part_parent.size(); part_parent.resize(leaves+n);
        for (int p=0; p<prev_count; ++p)
            for (int x=heads[p]; x>=0; x=next[x]) part_parent[leaves+x]=prev_start+p;
        Lca parts(part_parent);
        for (int i:groups[b]) {
            const auto& e=demands[i];
            int x=id[contracted.root(e.x)], y=id[contracted.root(e.y)];
            assert(x>=0 && y>=0 && x!=y);
            int meet=parts.query(leaves+x,leaves+y);
            assert(meet<leaves);
            out.index[i]=base+meet;
        }
    }
    return out;
}
