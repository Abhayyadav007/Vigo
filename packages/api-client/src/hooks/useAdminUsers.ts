import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { AdminUser, Page, UpdateRoleRequest, UserListQuery } from "@vigo/types";
import { useApiClient } from "../context";

export const adminUserKeys = {
  all: ["admin", "users"] as const,
  list: (query: UserListQuery, limit: number, offset: number) => [...adminUserKeys.all, query, limit, offset] as const,
};

/** `GET /v1/admin/users` */
export function useAdminUsers(query: UserListQuery, { limit = 20, offset = 0 } = {}) {
  const client = useApiClient();
  return useQuery({
    queryKey: adminUserKeys.list(query, limit, offset),
    queryFn: async () => {
      const { data } = await client.get<Page<AdminUser>>("/v1/admin/users", {
        params: { ...query, limit, offset },
      });
      return data;
    },
    placeholderData: keepPreviousData,
  });
}

/** `PATCH /v1/admin/users/{id}/role` */
export function useUpdateUserRole() {
  const client = useApiClient();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ userId, body }: { userId: string; body: UpdateRoleRequest }) => {
      const { data } = await client.patch<AdminUser>(`/v1/admin/users/${userId}/role`, body);
      return data;
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: adminUserKeys.all }),
  });
}
