#pragma once

struct Named { vector<W> lambda; W P; int trials, rounds, draws; };

namespace naming {
// Select rank k, using median-of-five medians to guarantee linear worst-case work.
inline W select(span<W> a, size_t k) {
    for (;;) {
        if (a.size() <= 5) {
            sort(a.begin(), a.end());
            return a[k];
        }
        size_t m = 0;
        for (size_t i = 0; i < a.size(); i += 5) {
            auto block = a.subspan(i, min(size_t(5), a.size() - i));
            sort(block.begin(), block.end());
            swap(a[m++], block[block.size()/2]);
        }
        W pivot = select(a.first(m), m/2);
        auto lo = partition(a.begin(), a.end(), [pivot](W w){ return w < pivot; });
        auto hi = partition(lo, a.end(), [pivot](W w){ return w == pivot; });
        size_t l = lo-a.begin(), r = hi-a.begin();
        if (k < l) a = a.first(l);
        else if (k < r) return pivot;
        else { a = a.subspan(r); k -= r; }
    }
}

inline W certificate(span<W> group, unsigned __int128 bound, W P) {
    W best = 0; // Zero is not an input weight.
    while (!group.empty()) {
        W median = select(group, group.size()/2);
        unsigned __int128 scaled = median;
        for (W f = P; f > 1; f >>= 1) scaled += scaled;
        if (scaled >= bound) {
            best = median;
            auto cut = partition(group.begin(), group.end(), [median](W w){ return w < median; });
            group = group.first(cut-group.begin());
        } else {
            auto cut = partition(group.begin(), group.end(), [median](W w){ return w <= median; });
            group = group.subspan(cut-group.begin());
        }
    }
    return best;
}
}

inline Named name_routes(int t, const vector<Edge>& edges, mt19937_64& rng, bool sample = true) {
    W u = bit_width(W(t)+1), g = 4*u+3;
    Named out{{}, bit_ceil(8*256*g*u*u), 0, 0, 0};
    vector<int> id(edges.size()), record(edges.size());
    iota(id.begin(), id.end(), 0);
    vector<pair<W,int>> records;
    auto append = [&](W w) {
        int k = int(records.size());
        records.emplace_back(w, k);
        return k;
    };
    vector<Edge> residual;
    const vector<Edge>* current = &edges;
    while (sample && id.size() > 32*size_t(t)) {
        const auto& demands = *current;
        ++out.trials;
        vector<Edge> picked;
        picked.reserve(16*t);
        std::sample(demands.begin(), demands.end(), back_inserter(picked), 16*t, rng);
        out.draws += int(picked.size());
        auto est = estimate(t, picked, demands, rng);

        // Contiguous, disjoint ranges for the estimator's connected groups.
        size_t groups = est.bounds.size();
        vector<size_t> start(groups+1);
        for (int j : est.index) if (j >= 0) ++start[j+1];
        partial_sum(start.begin(), start.end(), start.begin());
        vector<size_t> cursor(start.begin(), start.end()-1);
        vector<W> weights(start.back()), best(groups);
        for (size_t i = 0; i < demands.size(); ++i) {
            int j = est.index[i];
            if (j >= 0) weights[cursor[j]++] = demands[i].w;
        }
        for (size_t j = 0; j < groups; ++j)
            best[j] = naming::certificate(span(weights).subspan(start[j], start[j+1]-start[j]), est.bounds[j], out.P);
        auto qualifies = [&](size_t i) {
            int j = est.index[i];
            return j >= 0 && best[j] && demands[i].w >= best[j];
        };
        size_t qualified = 0;
        for (size_t i = 0; i < demands.size(); ++i) qualified += qualifies(i);
        if (qualified*2 < demands.size()) continue;

        ++out.rounds;
        vector<int> group_record(groups);
        for (size_t j = 0; j < groups; ++j) if (best[j]) group_record[j] = append(best[j]);
        size_t left = 0;
        if (current == &edges) residual.resize(demands.size()-qualified);
        for (size_t i = 0; i < demands.size(); ++i) {
            if (qualifies(i)) record[id[i]] = group_record[est.index[i]];
            else {
                residual[left] = demands[i];
                id[left++] = id[i];
            }
        }
        residual.resize(left);
        id.resize(left);
        current = &residual;
    }
    for (size_t i = 0; i < id.size(); ++i) record[id[i]] = append((*current)[i].w);

    sort(records.begin(), records.end());
    vector<W> dictionary;
    vector<int> index(records.size());
    for (auto [weight, old] : records) {
        if (dictionary.empty() || dictionary.back() != weight) dictionary.push_back(weight);
        index[old] = int(dictionary.size()-1);
    }
    out.lambda.reserve(edges.size());
    for (size_t i = 0; i < edges.size(); ++i) {
        W w = dictionary[index[record[i]]];
        assert(w > 0 && w <= edges[i].w);
        out.lambda.push_back(w);
    }
    return out;
}
