// Simulated async data fetching utilities
// Demonstrates: Promise creation, async functions, simulated network calls

// ============================================================================
// Types
// ============================================================================

interface User {
  id: number;
  name: string;
  email: string;
}

interface Post {
  id: number;
  userId: number;
  title: string;
  body: string;
}

interface Comment {
  id: number;
  postId: number;
  author: string;
  text: string;
}

// ============================================================================
// Simulated Database
// ============================================================================

const users: User[] = [
  { id: 1, name: "Alice", email: "alice@example.com" },
  { id: 2, name: "Bob", email: "bob@example.com" },
  { id: 3, name: "Charlie", email: "charlie@example.com" },
];

const posts: Post[] = [
  { id: 1, userId: 1, title: "Hello World", body: "My first post!" },
  { id: 2, userId: 1, title: "TypeScript Tips", body: "Use strict types." },
  { id: 3, userId: 2, title: "Async Patterns", body: "Promises are powerful." },
];

const comments: Comment[] = [
  { id: 1, postId: 1, author: "Bob", text: "Great post!" },
  { id: 2, postId: 1, author: "Charlie", text: "Welcome!" },
  { id: 3, postId: 2, author: "Bob", text: "Very helpful." },
  { id: 4, postId: 3, author: "Alice", text: "I agree!" },
];

// ============================================================================
// Async Fetch Functions
// ============================================================================

/**
 * Fetch a user by ID
 */
export async function fetchUser(id: number): Promise<User | null> {
  // Simulated async operation
  const user = users.find((u) => u.id === id);
  return user || null;
}

/**
 * Fetch all users
 */
export async function fetchAllUsers(): Promise<User[]> {
  return users;
}

/**
 * Fetch posts by user ID
 */
export async function fetchUserPosts(userId: number): Promise<Post[]> {
  return posts.filter((p) => p.userId === userId);
}

/**
 * Fetch a post by ID
 */
export async function fetchPost(id: number): Promise<Post | null> {
  const post = posts.find((p) => p.id === id);
  return post || null;
}

/**
 * Fetch comments for a post
 */
export async function fetchPostComments(postId: number): Promise<Comment[]> {
  return comments.filter((c) => c.postId === postId);
}

/**
 * Fetch user with their posts (demonstrates sequential awaits)
 */
export async function fetchUserWithPosts(
  userId: number
): Promise<{ user: User | null; posts: Post[] }> {
  const user = await fetchUser(userId);
  const userPosts = await fetchUserPosts(userId);
  return { user, posts: userPosts };
}

/**
 * Fetch multiple users in parallel (demonstrates Promise.all)
 */
export async function fetchUsersById(ids: number[]): Promise<(User | null)[]> {
  const promises = ids.map((id) => fetchUser(id));
  return Promise.all(promises);
}

/**
 * Fetch first available resource (demonstrates Promise.race pattern)
 */
export async function fetchFirstPost(postIds: number[]): Promise<Post | null> {
  const promises = postIds.map((id) => fetchPost(id));
  return Promise.race(promises);
}
