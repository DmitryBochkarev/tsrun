// Tree traversal generators
// Demonstrates: yield*, recursive generators, custom iterables

/**
 * Tree node interface
 */
interface TreeNode<T> {
  value: T;
  children: TreeNode<T>[];
}

/**
 * Create a tree node
 */
export function createNode<T>(value: T, children: TreeNode<T>[] = []): TreeNode<T> {
  return { value, children };
}

/**
 * Pre-order traversal (parent before children)
 */
export function* preorder<T>(node: TreeNode<T>): Generator<T> {
  yield node.value;
  for (const child of node.children) {
    yield* preorder(child);
  }
}

/**
 * Post-order traversal (children before parent)
 */
export function* postorder<T>(node: TreeNode<T>): Generator<T> {
  for (const child of node.children) {
    yield* postorder(child);
  }
  yield node.value;
}

/**
 * Level-order traversal (breadth-first)
 */
export function* levelOrder<T>(root: TreeNode<T>): Generator<T> {
  const queue: TreeNode<T>[] = [root];
  while (queue.length > 0) {
    const node = queue.shift();
    if (node) {
      yield node.value;
      for (const child of node.children) {
        queue.push(child);
      }
    }
  }
}

/**
 * Get all leaf nodes (nodes with no children)
 */
export function* leaves<T>(node: TreeNode<T>): Generator<T> {
  if (node.children.length === 0) {
    yield node.value;
  } else {
    for (const child of node.children) {
      yield* leaves(child);
    }
  }
}

/**
 * Flatten a nested array using generator
 */
export function* flatten<T>(arr: (T | T[])[]): Generator<T> {
  for (const item of arr) {
    if (Array.isArray(item)) {
      yield* flatten(item as (T | T[])[]);
    } else {
      yield item;
    }
  }
}
