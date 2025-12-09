// Graph implementation using Map<node, Set<neighbor>>

export interface Graph<T> {
    nodes: Map<T, Set<T>>;
}

export function createGraph<T>(): Graph<T> {
    return {
        nodes: new Map()
    };
}

export function addNode<T>(graph: Graph<T>, node: T): void {
    if (!graph.nodes.has(node)) {
        graph.nodes.set(node, new Set());
    }
}

export function addEdge<T>(graph: Graph<T>, from: T, to: T): void {
    addNode(graph, from);
    addNode(graph, to);
    graph.nodes.get(from)!.add(to);
}

export function addBidirectionalEdge<T>(graph: Graph<T>, a: T, b: T): void {
    addEdge(graph, a, b);
    addEdge(graph, b, a);
}

export function hasEdge<T>(graph: Graph<T>, from: T, to: T): boolean {
    const neighbors: Set<T> | undefined = graph.nodes.get(from);
    return neighbors !== undefined && neighbors.has(to);
}

export function getNeighbors<T>(graph: Graph<T>, node: T): Set<T> {
    return graph.nodes.get(node) || new Set();
}

export function removeEdge<T>(graph: Graph<T>, from: T, to: T): void {
    const neighbors: Set<T> | undefined = graph.nodes.get(from);
    if (neighbors) {
        neighbors.delete(to);
    }
}

export function removeNode<T>(graph: Graph<T>, node: T): void {
    graph.nodes.delete(node);
    // Remove all edges pointing to this node
    graph.nodes.forEach((neighbors) => {
        neighbors.delete(node);
    });
}

// Breadth-First Search
export function bfs<T>(graph: Graph<T>, start: T): T[] {
    const visited: Set<T> = new Set();
    const result: T[] = [];
    const queue: T[] = [start];

    while (queue.length > 0) {
        const current: T = queue.shift()!;

        if (visited.has(current)) {
            continue;
        }

        visited.add(current);
        result.push(current);

        const neighbors: Set<T> = getNeighbors(graph, current);
        for (const neighbor of neighbors) {
            if (!visited.has(neighbor)) {
                queue.push(neighbor);
            }
        }
    }

    return result;
}

// Depth-First Search
export function dfs<T>(graph: Graph<T>, start: T): T[] {
    const visited: Set<T> = new Set();
    const result: T[] = [];

    function visit(node: T): void {
        if (visited.has(node)) {
            return;
        }

        visited.add(node);
        result.push(node);

        const neighbors: Set<T> = getNeighbors(graph, node);
        for (const neighbor of neighbors) {
            visit(neighbor);
        }
    }

    visit(start);
    return result;
}

// Find path between two nodes
export function findPath<T>(graph: Graph<T>, start: T, end: T): T[] | null {
    const visited: Set<T> = new Set();
    const parent: Map<T, T> = new Map();
    const queue: T[] = [start];

    visited.add(start);

    while (queue.length > 0) {
        const current: T = queue.shift()!;

        if (current === end) {
            // Reconstruct path
            const path: T[] = [];
            let node: T | undefined = end;
            while (node !== undefined) {
                path.unshift(node);
                node = parent.get(node);
            }
            return path;
        }

        const neighbors: Set<T> = getNeighbors(graph, current);
        for (const neighbor of neighbors) {
            if (!visited.has(neighbor)) {
                visited.add(neighbor);
                parent.set(neighbor, current);
                queue.push(neighbor);
            }
        }
    }

    return null;  // No path found
}

// Check if graph has a cycle (for directed graphs)
export function hasCycle<T>(graph: Graph<T>): boolean {
    const visited: Set<T> = new Set();
    const inStack: Set<T> = new Set();

    function dfsCheck(node: T): boolean {
        visited.add(node);
        inStack.add(node);

        const neighbors: Set<T> = getNeighbors(graph, node);
        for (const neighbor of neighbors) {
            if (!visited.has(neighbor)) {
                if (dfsCheck(neighbor)) {
                    return true;
                }
            } else if (inStack.has(neighbor)) {
                return true;  // Found cycle
            }
        }

        inStack.delete(node);
        return false;
    }

    for (const node of graph.nodes.keys()) {
        if (!visited.has(node)) {
            if (dfsCheck(node)) {
                return true;
            }
        }
    }

    return false;
}

// Build a sample graph
export function buildGraph(): Graph<string> {
    const graph: Graph<string> = createGraph();

    // Add edges to create:
    //     A
    //    / \
    //   B   C
    //  / \   \
    // D   E   F

    addEdge(graph, "A", "B");
    addEdge(graph, "A", "C");
    addEdge(graph, "B", "D");
    addEdge(graph, "B", "E");
    addEdge(graph, "C", "F");

    return graph;
}
