import { useQuery, useQueryClient } from "@tanstack/react-query";
import { api, type User } from "@/lib/api";

/** Loads the current user; `null` when not logged in. */
export function useCurrentUser() {
  return useQuery({
    queryKey: ["me"],
    queryFn: async () => {
      try {
        return await api.get<User>("/api/me");
      } catch {
        return null;
      }
    },
  });
}

export function useInvalidateUser() {
  const qc = useQueryClient();
  return () => qc.invalidateQueries({ queryKey: ["me"] });
}
