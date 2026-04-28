// Local-first stub. The original hook fetched stargazer count from
// `BloopAI/gencap` to decorate the upstream-community Star button —
// that button is gone, and the upstream repo no longer exists for
// this fork. Returning a static null preserves the consumer shape
// without firing 404s in the console.
export function useGitHubStars() {
  return {
    data: null as number | null,
    isLoading: false,
    isError: false,
    error: null,
  };
}
