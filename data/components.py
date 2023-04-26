#!/usr/bin/env python3
import random
import networkx as nx
import matplotlib.pyplot as plt
import seaborn as sns

from cycler import cycler
import matplotlib as mpl


class DisJointSets():
    def __init__(self, N):
        # Initially, all elements are single element subsets
        self._parents = [node for node in range(N)]
        self._ranks = [1 for _ in range(N)]

    def find(self, u):
        while u != self._parents[u]:
            # path compression technique
            self._parents[u] = self._parents[self._parents[u]]
            u = self._parents[u]
        return u

    def connected(self, u, v):
        return self.find(u) == self.find(v)

    def union(self, u, v):
        # Union by rank optimization
        root_u, root_v = self.find(u), self.find(v)
        if root_u == root_v:
            return root_u

        if self._ranks[root_u] > self._ranks[root_v]:
            self._parents[root_v] = root_u
            return root_u

        elif self._ranks[root_v] > self._ranks[root_u]:
            self._parents[root_u] = root_v
            return root_v

        else:
            self._parents[root_u] = root_v
            self._ranks[root_v] += 1
            return root_v

    def cc_sizes(self):
        # returns a list of sizes of connected components
        cc_sizes = {}
        for parent in self._parents:
            if parent in cc_sizes:
                cc_sizes[parent] += 1
            else:
                cc_sizes[parent] = 1
        return sorted(cc_sizes.values())

    def nodes_in(self, cc):
        # returns a list of nodes in a connected component
        return [node for node in range(len(self._parents)) if self.find(node) == cc]


nodes = 50
max_edges = nodes * (nodes - 1) // 4
edges = set()

cmap = plt.get_cmap('tab20b', nodes // 2)
colors = [mpl.colors.rgb2hex(cmap(i)) for i in range(cmap.N)]

ccs = DisJointSets(nodes)
graph = nx.Graph()
graph.add_nodes_from(range(nodes))

COLOR_DISCONNECT = (0.8, 0.8, 0.8)
nx.set_node_attributes(graph, COLOR_DISCONNECT, 'color')
nx.set_edge_attributes(graph, COLOR_DISCONNECT, 'color')

pos = nx.layout.circular_layout(graph)

color_req = 0
while len(edges) < max_edges:
    u, v = random.randint(0, nodes - 1), random.randint(0, nodes - 1)
    if u == v or (u, v) in edges:
        continue

    c1 = graph.nodes[ccs.find(u)]['color']
    c2 = graph.nodes[ccs.find(v)]['color']
    r = ccs.union(u, v)

    if c1 == COLOR_DISCONNECT and c2 == COLOR_DISCONNECT:
        color_req += 4
        color = colors[color_req % len(colors)]
    elif c1 == COLOR_DISCONNECT:
        color = c2
    else:
        color = c1

    for node in ccs.nodes_in(r):
        graph.nodes[node]['color'] = color

    edges.add((u, v))
    graph.add_edge(u, v, color=color)

    plt.clf()
    fig, (gax, hax) = plt.subplots(nrows=1, ncols=2, figsize=(12, 6))
    gax.set_title("Knoten n={}, Kanten m={}".format(
        nodes, graph.number_of_edges()))
    nx.draw(graph, with_labels=True, ax=gax, pos=pos,
            node_color=[graph.nodes[u]["color"]
                        for u in graph.nodes()],
            edge_color=[graph.nodes[u]["color"] for (u, v) in graph.edges()])

    ccs_sizes = ccs.cc_sizes()
    sns.histplot(ccs_sizes, ax=hax, bins=nodes, discrete=True)
    hax.set_xlim(0, nodes+1)
    hax.set_ylim(0, nodes)
    hax.set_xlabel("Größe der Komponenten")
    hax.set_ylabel("Anzahl der Komponenten")

    plt.savefig("animation/slide%04d.png" % (len(edges)))
    plt.close()
    print(len(edges), len(ccs_sizes))

    if len(ccs_sizes) == 1:
        break
