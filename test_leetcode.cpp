class Solution {
public:
    vector<vector<int>> buildMatrix(int k, vector<vector<int>>& rowConditions, vector<vector<int>>& colConditions) {
        auto topo_sort = [&](vector<vector<int>> edge) -> vector<int> {
            vector adj(k, vector<int> {});
            vector indegree(k, 0);
            for (int i = 0; i < edge.size(); ++i) {
                int x = edge[i][0] - 1, y = edge[i][1] - 1;
                adj[x].emplace_back(y);
                indegree[y]++;
            }

            vector<int> order;
            queue<int> q;
            for (int i = 0; i < k; ++i) {
                if (indegree[i] == 0) {
                    q.emplace(i);
                    order.emplace_back(i);
                }
            }

            while (!q.empty()) {
                int u = q.front(); q.pop();

                for (int& v : adj[u]) {
                    if (--indegree[v] == 0) {
                        q.emplace(v);
                        order.emplace_back(v);
                    }
                }
            }

            return (order.size() == k ? order : vector<int>());
        };

        auto row = topo_sort(rowConditions);
        if (row.size() == 0) return {};
        auto col = topo_sort(colConditions);
        if (col.size() == 0) return {};

        vector pos(k, 0);
        for (int i = 0; i < k; ++i) pos[col[i]] = i;

        vector ans(k, vector(k, 0));
        for (int i = 0; i < k; ++i) ans[i][pos[row[i]]] = row[i]+1;

        return ans;
    }
};
