// Async/Await Demo - Finding the bug
import {
  fetchUser,
  fetchAllUsers,
  fetchUserPosts,
  fetchPostComments,
  fetchUsersById,
  fetchUserWithPosts,
} from "./fetcher";
import {
  safeProcess,
  asyncMap,
  calculateStats,
  pipeline,
  processAllSettled,
} from "./processor";

// Demo 1: Basic
async function basicAsyncDemo(): Promise<{ name: string; postCount: number }> {
  const user = await fetchUser(1);
  const posts = await fetchUserPosts(1);
  return { name: user ? user.name : "Unknown", postCount: posts.length };
}

// Demo 2: Parallel
async function parallelDemo(): Promise<{ totalFetched: number }> {
  const users = await fetchUsersById([1, 2, 3]);
  return { totalFetched: users.filter((u) => u !== null).length };
}

// Demo 3: Sequential
async function sequentialFetch(): Promise<{ user1Posts: number; user2Posts: number }> {
  const user1Posts = await fetchUserPosts(1);
  const user2Posts = await fetchUserPosts(2);
  return { user1Posts: user1Posts.length, user2Posts: user2Posts.length };
}

// Demo 4: Parallel Fetch - TESTING THIS (Promise.all with destructuring)
async function parallelFetch(): Promise<{ user1Posts: number; user2Posts: number }> {
  const [user1Posts, user2Posts] = await Promise.all([
    fetchUserPosts(1),
    fetchUserPosts(2),
  ]);
  return {
    user1Posts: user1Posts.length,
    user2Posts: user2Posts.length,
  };
}

// Demo 5: Pipeline
async function pipelineDemo(): Promise<string[]> {
  return pipeline(
    fetchUser(1),
    async (user) => {
      if (!user) return [];
      return fetchUserPosts(user.id);
    },
    async (posts) => posts.map((p) => p.title)
  );
}

// Demo 6: allSettled
async function allSettledDemo(): Promise<{ fulfilled: number; rejected: number }> {
  const { fulfilled, rejected } = await processAllSettled([1, 2, 99], async (id: number) => {
    const user = await fetchUser(id);
    if (!user) throw new Error("Not found");
    return user;
  });
  return { fulfilled: fulfilled.length, rejected: rejected.length };
}

// Demo 7: Async map
async function asyncMapDemo(): Promise<string[]> {
  return asyncMap([1, 2, 3], async (id) => {
    const user = await fetchUser(id);
    return user ? user.name : "Unknown";
  });
}

// Demo 8: Stats
async function statsDemo(): Promise<{ userCount: number }> {
  const stats = await calculateStats(fetchAllUsers, (user) => user.name);
  return { userCount: stats.count };
}

// Demo 9: Safe process
async function safeProcessDemo(): Promise<{ success1: boolean; success2: boolean }> {
  const result1 = await safeProcess([1, 2, 3], (arr: number[]) => arr.reduce((a, b) => a + b, 0));
  const result2 = await safeProcess(5, (n: number) => {
    if (n > 3) throw new Error("too big");
    return n;
  });
  return { success1: result1.success, success2: result2.success };
}

// Demo 10: Nested
async function nestedAsyncDemo(): Promise<{
  userName: string;
  postTitles: string[];
  commentCount: number;
}> {
  const { user, posts } = await fetchUserWithPosts(1);
  if (!user) {
    return { userName: "Unknown", postTitles: [], commentCount: 0 };
  }
  const allComments = await Promise.all(
    posts.map((post) => fetchPostComments(post.id))
  );
  return {
    userName: user.name,
    postTitles: posts.map((p) => p.title),
    commentCount: allComments.flat().length,
  };
}

// Demo 11: Error handling
async function errorHandlingDemo(): Promise<{ result: string; caught: boolean }> {
  let caught = false;
  try {
    const user = await fetchUser(1);
    if (!user) {
      throw new Error("User not found");
    }
    return { result: user.name, caught };
  } catch (e) {
    caught = true;
    return { result: "Error occurred", caught };
  }
}

// Run all
async function runAllDemos(): Promise<{
  basic: { name: string; postCount: number };
  parallel: { totalFetched: number };
  sequential: { user1Posts: number; user2Posts: number };
  parallelFetch: { user1Posts: number; user2Posts: number };
  pipeline: string[];
  allSettled: { fulfilled: number; rejected: number };
  asyncMap: string[];
  stats: { userCount: number };
  safeProcess: { success1: boolean; success2: boolean };
  nested: { userName: string; postTitles: string[]; commentCount: number };
  errorHandling: { result: string; caught: boolean };
}> {
  return {
    basic: await basicAsyncDemo(),
    parallel: await parallelDemo(),
    sequential: await sequentialFetch(),
    parallelFetch: await parallelFetch(),
    pipeline: await pipelineDemo(),
    allSettled: await allSettledDemo(),
    asyncMap: await asyncMapDemo(),
    stats: await statsDemo(),
    safeProcess: await safeProcessDemo(),
    nested: await nestedAsyncDemo(),
    errorHandling: await errorHandlingDemo(),
  };
}

const results = await runAllDemos();
JSON.stringify(results, null, 2);
