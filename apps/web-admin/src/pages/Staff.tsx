import { ApiError, formatIndianPhone, useAdminUsers, useUpdateUserRole } from "@vigo/api-client";
import type { AdminUser, Role } from "@vigo/types";
import { useDeferredValue, useState } from "react";

const ROLES: Role[] = ["CUSTOMER", "PICKER", "RIDER", "ADMIN"];
const PAGE_SIZE = 20;

export function Staff({ currentUserId }: { currentUserId: string }) {
  const [phone, setPhone] = useState("");
  const [role, setRole] = useState<Role | "">("");
  const [offset, setOffset] = useState(0);
  const phoneFilter = useDeferredValue(phone.replace(/[^\d+]/g, ""));

  const users = useAdminUsers(
    { ...(phoneFilter ? { phone: phoneFilter } : {}), ...(role ? { role } : {}) },
    { limit: PAGE_SIZE, offset },
  );
  const total = users.data?.total ?? 0;

  return (
    <section className="card">
      <h2>Users &amp; staff</h2>
      <div className="filters">
        <input
          placeholder="Search phone"
          value={phone}
          onChange={(e) => {
            setPhone(e.target.value);
            setOffset(0);
          }}
        />
        <select
          value={role}
          onChange={(e) => {
            setRole(e.target.value as Role | "");
            setOffset(0);
          }}
        >
          <option value="">All roles</option>
          {ROLES.map((r) => (
            <option key={r}>{r}</option>
          ))}
        </select>
      </div>

      {users.isError ? <p className="error">{users.error.message}</p> : null}
      <table>
        <thead>
          <tr>
            <th>Phone</th>
            <th>Role</th>
            <th>Store</th>
            <th>Joined</th>
          </tr>
        </thead>
        <tbody>
          {users.data?.items.map((u) => (
            <UserRow key={u.id} user={u} isSelf={u.id === currentUserId} />
          ))}
          {users.data && users.data.items.length === 0 ? (
            <tr>
              <td colSpan={4} className="muted">
                No users match.
              </td>
            </tr>
          ) : null}
        </tbody>
      </table>

      <div className="pager">
        <span className="muted">
          {total === 0 ? "0" : `${offset + 1}–${Math.min(offset + PAGE_SIZE, total)}`} of {total}
        </span>
        <button className="btn secondary" disabled={offset === 0} onClick={() => setOffset(offset - PAGE_SIZE)}>
          Previous
        </button>
        <button
          className="btn secondary"
          disabled={offset + PAGE_SIZE >= total}
          onClick={() => setOffset(offset + PAGE_SIZE)}
        >
          Next
        </button>
      </div>
    </section>
  );
}

function UserRow({ user, isSelf }: { user: AdminUser; isSelf: boolean }) {
  const update = useUpdateUserRole();
  const error = update.error instanceof ApiError ? update.error.message : update.error?.message;

  return (
    <tr data-testid={`user-${user.phone}`}>
      <td>{formatIndianPhone(user.phone)}</td>
      <td>
        <select
          aria-label={`Role for ${user.phone}`}
          value={user.role}
          disabled={isSelf || update.isPending}
          title={isSelf ? "You can't change your own role" : undefined}
          onChange={(e) => update.mutate({ userId: user.id, body: { role: e.target.value as Role } })}
        >
          {ROLES.map((r) => (
            <option key={r}>{r}</option>
          ))}
        </select>
        {error ? <div className="error small">{error}</div> : null}
      </td>
      {/* TODO(phase-3): store picker once dark stores exist. */}
      <td className="muted">{user.storeId ?? "—"}</td>
      <td className="muted">{new Date(user.createdAt).toLocaleDateString("en-IN")}</td>
    </tr>
  );
}
