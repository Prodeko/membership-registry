import {
  QueryKey,
  useAddMultipleRolesToMembers,
  useAssignRoleGroup,
  useGetMember,
  useGetMemberRoleGroups,
  useGetMemberRoles,
  useGetRoleGroups,
  useGetRoles,
  useRemoveMemberRole,
  useRemoveRoleGroupAssignment,
  useUpdateMember,
} from "@/lib/api";
import { useQueryClient } from "@tanstack/react-query";
import { ChevronDown, Pencil, Plus, Trash2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Badge } from "../ui/badge";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { DateRangePicker } from "../ui/date-range-picker";
import RoleBadge from "../ui/role-badge";
import DeleteMembersModal from "./DeleteMembersModal";
import MunicipalitySelect from "../ui/MunicipalitySelect";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "../ui/form";
import { COUNTRIES, FINNISH_MUNICIPALITIES } from "@/lib/constants";
import { cn } from "@/lib/utils";

// ── helpers ───────────────────────────────────────────────────────────────────

const fmtDate = (d: string | null) => {
  if (!d) return "—";
  const [y, m, day] = d.split("-");
  return `${day}.${m}.${y}`;
};

const dateToStr = (d: Date) => {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
};

const today = new Date().toISOString().split("T")[0];

const validityStatus = (validFrom: string, validUntil: string | null) => {
  if (!validUntil) return "active";
  if (validUntil < today) return "expired";
  if (validFrom > today) return "future";
  return "active";
};

const ValidityPill = ({
  validFrom,
  validUntil,
}: {
  validFrom: string;
  validUntil: string | null;
}) => {
  const status = validityStatus(validFrom, validUntil);
  const cfg = {
    active: "bg-green-50 text-green-700 border-green-200",
    expired: "bg-red-50 text-red-700 border-red-200",
    future: "bg-slate-50 text-slate-600 border-slate-200",
  }[status];
  const dot = {
    active: "bg-green-500",
    expired: "bg-red-500",
    future: "bg-slate-400",
  }[status];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 text-xs font-medium px-2 py-0.5 rounded-full border",
        cfg,
      )}
    >
      <span className={cn("w-1.5 h-1.5 rounded-full", dot)} />
      {fmtDate(validFrom)} – {fmtDate(validUntil)}
    </span>
  );
};

// ── profile schema ────────────────────────────────────────────────────────────

const profileSchema = z.object({
  first_name: z.string().min(1),
  last_name: z.string().min(1),
  email: z.string().email(),
  home_municipality: z
    .enum([...FINNISH_MUNICIPALITIES, ...COUNTRIES] as [string, ...string[]])
    .optional(),
});
type ProfileValues = z.infer<typeof profileSchema>;

// ── staged types ──────────────────────────────────────────────────────────────

type GroupAddition = { groupId: string; validFrom: string; validUntil: string };
type RoleAddition = { roleName: string; validFrom: string; validUntil: string };

interface MemberDrawerProps {
  userId: string;
  onClose: () => void;
}

// ── component ─────────────────────────────────────────────────────────────────

export default function MemberDrawer({ userId, onClose }: MemberDrawerProps) {
  const queryClient = useQueryClient();

  // ── data ──
  const { data: member } = useGetMember(userId);
  const { data: memberRoles } = useGetMemberRoles(userId);
  const { data: memberGroupMemberships } = useGetMemberRoleGroups(userId);
  const { data: allRoles } = useGetRoles();
  const { data: allGroups } = useGetRoleGroups();

  // ── mutations ──
  const { mutateAsync: updateMember } = useUpdateMember();
  const { mutateAsync: assignGroup } = useAssignRoleGroup();
  const { mutateAsync: removeGroup } = useRemoveRoleGroupAssignment();
  const { mutateAsync: addRoles } = useAddMultipleRolesToMembers();
  const { mutateAsync: removeRole } = useRemoveMemberRole();

  // ── local staged state ──
  const [editingProfile, setEditingProfile] = useState(false);
  const [groupAdditions, setGroupAdditions] = useState<GroupAddition[]>([]);
  const [groupRemovals, setGroupRemovals] = useState<
    Set<string> // key = `${groupId}::${validFrom}`
  >(new Set());
  const [groupDateEdits, setGroupDateEdits] = useState<
    Record<string, { validFrom: string; validUntil: string }>
  >({}); // key = groupId
  const [roleAdditions, setRoleAdditions] = useState<RoleAddition[]>([]);
  const [roleRemovals, setRoleRemovals] = useState<
    Set<string> // key = `${roleName}::${validFrom}`
  >(new Set());
  const [roleDateEdits, setRoleDateEdits] = useState<
    Record<string, { validFrom: string; validUntil: string }>
  >({}); // key = `${roleName}::${validFrom}`

  // ── UI state ──
  const [expandedGroups, setExpandedGroups] = useState<Record<string, boolean>>(
    {},
  );
  const [expandedRoles, setExpandedRoles] = useState<Record<string, boolean>>(
    {},
  );
  const [showAddGroup, setShowAddGroup] = useState(false);
  const [showAddRole, setShowAddRole] = useState(false);
  const [addGroupId, setAddGroupId] = useState("");
  const [addGroupFrom, setAddGroupFrom] = useState(today);
  const [addGroupUntil, setAddGroupUntil] = useState("");
  const [addRoleName, setAddRoleName] = useState("");
  const [addRoleFrom, setAddRoleFrom] = useState(today);
  const [addRoleUntil, setAddRoleUntil] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isClosing, setIsClosing] = useState(false);

  // ── profile form ──
  const form = useForm<ProfileValues>({
    resolver: zodResolver(profileSchema),
    values: member
      ? {
          first_name: member.first_name,
          last_name: member.last_name,
          email: member.email,
          home_municipality:
            (member.home_municipality as ProfileValues["home_municipality"]) ??
            undefined,
        }
      : undefined,
  });

  // ── close handler with animation ──
  const handleClose = () => {
    setIsClosing(true);
    setTimeout(() => {
      onClose();
    }, 250);
  };

  // ── close on Escape ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [handleClose]);

  // ── computed ──
  const initials = member
    ? (member.first_name[0] + member.last_name[0]).toUpperCase()
    : "??";

  const removalKey = (a: string, b: string) => `${a}::${b}`;

  // Active group memberships (server data minus pending removals)
  const activeGroupMemberships = useMemo(() => {
    return (memberGroupMemberships ?? []).filter(
      (gm) => !groupRemovals.has(removalKey(gm.group_id, gm.valid_from)),
    );
  }, [memberGroupMemberships, groupRemovals]);

  // Active roles (server data minus pending removals)
  const activeRoles = useMemo(() => {
    return (memberRoles ?? []).filter(
      (r) => !roleRemovals.has(removalKey(r.role_name, r.valid_from)),
    );
  }, [memberRoles, roleRemovals]);

  // All group memberships including staged additions
  const allGroupMemberships = useMemo(() => {
    const memberships = [...activeGroupMemberships];
    groupAdditions.forEach((ga) => {
      if (!groupRemovals.has(removalKey(ga.groupId, ga.validFrom))) {
        const group = allGroups?.find((g) => g.id === ga.groupId);
        memberships.push({
          group_id: ga.groupId,
          group_name: group?.name ?? ga.groupId,
          valid_from: ga.validFrom,
          valid_until: ga.validUntil,
          user_id: userId,
        });
      }
    });
    return memberships;
  }, [activeGroupMemberships, groupAdditions, groupRemovals, allGroups, userId]);

  // All roles including staged additions
  const allActiveRoles = useMemo(() => {
    const roles = [...activeRoles];
    roleAdditions.forEach((ra) => {
      if (!roleRemovals.has(removalKey(ra.roleName, ra.validFrom))) {
        roles.push({
          role_name: ra.roleName,
          valid_from: ra.validFrom,
          valid_until: ra.validUntil,
          user_id: userId,
          renewable: false,
          renewal_payment_link: null,
          pending_renewal_id: null,
        });
      }
    });
    return roles;
  }, [activeRoles, roleAdditions, roleRemovals, userId]);

  // Set of role names covered by active groups
  const groupRoleSet = useMemo(() => {
    const set = new Set<string>();
    activeGroupMemberships.forEach((gm) => {
      const group = allGroups?.find((g) => g.id === gm.group_id);
      group?.role_names.forEach((r) => set.add(r));
    });
    groupAdditions.forEach((ga) => {
      const group = allGroups?.find((g) => g.id === ga.groupId);
      group?.role_names.forEach((r) => set.add(r));
    });
    return set;
  }, [activeGroupMemberships, groupAdditions, allGroups]);

  // Effective roles for summary section
  const effectiveRoles = useMemo(() => {
    const map: Record<string, { fromGroups: string[]; fromDirect: boolean }> =
      {};
    [
      ...activeGroupMemberships,
      ...groupAdditions.map((ga) => ({
        group_id: ga.groupId,
        group_name: allGroups?.find((g) => g.id === ga.groupId)?.name ?? "",
        valid_from: ga.validFrom,
        valid_until: ga.validUntil,
        user_id: userId,
      })),
    ].forEach((gm) => {
      const group = allGroups?.find((g) => g.id === gm.group_id);
      if (!group) return;
      group.role_names.forEach((r) => {
        if (!map[r]) map[r] = { fromGroups: [], fromDirect: false };
        if (!map[r].fromGroups.includes(group.name))
          map[r].fromGroups.push(group.name);
      });
    });
    [
      ...activeRoles,
      ...roleAdditions.map((ra) => ({
        role_name: ra.roleName,
        valid_from: ra.validFrom,
        valid_until: ra.validUntil,
        user_id: userId,
        renewable: false,
        renewal_payment_link: null,
        pending_renewal_id: null,
      })),
    ].forEach((r) => {
      if (!map[r.role_name])
        map[r.role_name] = { fromGroups: [], fromDirect: false };
      map[r.role_name].fromDirect = true;
    });
    return map;
  }, [
    activeGroupMemberships,
    groupAdditions,
    activeRoles,
    roleAdditions,
    allGroups,
    userId,
  ]);

  // Available groups not yet assigned
  const availableGroups = useMemo(() => {
    const assignedIds = new Set([
      ...activeGroupMemberships.map((gm) => gm.group_id),
      ...groupAdditions.map((ga) => ga.groupId),
    ]);
    return (allGroups ?? []).filter((g) => !assignedIds.has(g.id));
  }, [allGroups, activeGroupMemberships, groupAdditions]);

  // Available roles not yet directly assigned
  const availableRoles = useMemo(() => {
    const assignedNames = new Set([
      ...activeRoles.map((r) => r.role_name),
      ...roleAdditions.map((ra) => ra.roleName),
    ]);
    return (allRoles ?? []).filter((r) => !assignedNames.has(r.name));
  }, [allRoles, activeRoles, roleAdditions]);

  // Active roles grouped by role name, each group sorted oldest→latest
  const groupedActiveRoles = useMemo(() => {
    const map: Record<string, typeof allActiveRoles> = {};
    allActiveRoles.forEach((r) => {
      if (!map[r.role_name]) map[r.role_name] = [];
      map[r.role_name].push(r);
    });
    Object.values(map).forEach((g) =>
      g.sort((a, b) => a.valid_from.localeCompare(b.valid_from)),
    );
    return map;
  }, [allActiveRoles]);

  // ── handlers ──

  const handleAddGroup = () => {
    if (!addGroupId) return;
    setGroupAdditions((p) => [
      ...p,
      {
        groupId: addGroupId,
        validFrom: addGroupFrom,
        validUntil: addGroupUntil,
      },
    ]);
    setShowAddGroup(false);
    setAddGroupId("");
    setAddGroupFrom(today);
    setAddGroupUntil("");
  };

  const handleRemoveGroup = (groupId: string, validFrom: string) => {
    setGroupRemovals((p) => new Set([...p, removalKey(groupId, validFrom)]));
  };


  const handleAddRole = () => {
    if (!addRoleName) return;
    setRoleAdditions((p) => [
      ...p,
      {
        roleName: addRoleName,
        validFrom: addRoleFrom,
        validUntil: addRoleUntil,
      },
    ]);
    setShowAddRole(false);
    setAddRoleName("");
    setAddRoleFrom(today);
    setAddRoleUntil("");
  };

  const handleRemoveRole = (roleName: string, validFrom: string) => {
    setRoleRemovals((p) => new Set([...p, removalKey(roleName, validFrom)]));
  };


  const handleSave = async () => {
    setIsSaving(true);
    try {
      // 1. Profile
      if (editingProfile) {
        const vals = form.getValues();
        await updateMember({
          userId,
          data: {
            first_name: vals.first_name,
            last_name: vals.last_name,
            email: vals.email,
            home_municipality: vals.home_municipality ?? null,
            email_notifications: member?.email_notifications ?? true,
            language: member?.language ?? "fi",
          },
        });
        setEditingProfile(false);
      }

      // 2. Remove groups
      for (const key of groupRemovals) {
        const [groupId, validFrom] = key.split("::");
        await removeGroup({ groupId, userId, validFrom });
      }

      // 3. Update group dates (delete + re-add)
      for (const [groupId, dates] of Object.entries(groupDateEdits)) {
        const original = memberGroupMemberships?.find(
          (gm) => gm.group_id === groupId,
        );
        if (!original) continue;
        if (groupRemovals.has(removalKey(groupId, original.valid_from)))
          continue; // already removed
        await removeGroup({
          groupId,
          userId,
          validFrom: original.valid_from,
        });
        await assignGroup({
          groupId,
          userId,
          validFrom: dates.validFrom || undefined,
          validUntil: dates.validUntil || undefined,
        });
      }

      // 4. Add groups
      for (const ga of groupAdditions) {
        if (groupRemovals.has(removalKey(ga.groupId, ga.validFrom))) continue;
        const editKey = ga.groupId;
        const dateEdit = groupDateEdits[editKey];
        const validUntil = dateEdit?.validUntil ?? ga.validUntil;
        await assignGroup({
          groupId: ga.groupId,
          userId,
          validFrom: (dateEdit?.validFrom ?? ga.validFrom) || undefined,
          validUntil: validUntil || undefined,
        });
      }

      // 5. Remove roles
      for (const key of roleRemovals) {
        const [roleName, validFrom] = key.split("::");
        await removeRole({ userId, roleName, validFrom });
      }

      // 6. Update role dates (delete + re-add)
      for (const [key, dates] of Object.entries(roleDateEdits)) {
        const [roleName, originalFrom] = key.split("::");
        if (roleRemovals.has(removalKey(roleName, originalFrom))) continue;
        await removeRole({ userId, roleName, validFrom: originalFrom });
        await addRoles({
          userIds: [userId],
          roleNames: [roleName],
          validFrom: new Date(dates.validFrom),
          validUntil: new Date(dates.validUntil),
        });
      }

      // 7. Add roles
      for (const ra of roleAdditions) {
        if (roleRemovals.has(removalKey(ra.roleName, ra.validFrom))) continue;
        const editKey = `${ra.roleName}::${ra.validFrom}`;
        const dateEdit = roleDateEdits[editKey];
        const validUntil = dateEdit?.validUntil ?? ra.validUntil;
        await addRoles({
          userIds: [userId],
          roleNames: [ra.roleName],
          validFrom: new Date(dateEdit?.validFrom ?? ra.validFrom),
          validUntil: validUntil
            ? new Date(validUntil)
            : new Date("2099-12-31"),
        });
      }

      // 8. Invalidate
      await queryClient.invalidateQueries({ queryKey: [QueryKey.MEMBER] });
      await queryClient.invalidateQueries({
        queryKey: [QueryKey.MEMBER_ROLES],
      });
      await queryClient.invalidateQueries({
        queryKey: [QueryKey.ROLE_GROUPS],
      });
      await queryClient.invalidateQueries({
        queryKey: [QueryKey.MEMBERS_WITH_ROLES],
      });

      handleClose();
    } finally {
      setIsSaving(false);
    }
  };

  const hasChanges =
    editingProfile ||
    groupAdditions.length > 0 ||
    groupRemovals.size > 0 ||
    Object.keys(groupDateEdits).length > 0 ||
    roleAdditions.length > 0 ||
    roleRemovals.size > 0 ||
    Object.keys(roleDateEdits).length > 0;

  if (!member) return null;

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={handleClose}
        className={cn(
          "fixed inset-0 z-[199] bg-black/30 backdrop-blur-[1px] duration-200",
          isClosing ? "animate-out fade-out opacity-0" : "animate-in fade-in",
        )}
      />

      {/* Drawer */}
      <div
        className={cn(
          "fixed top-0 right-0 bottom-0 w-[480px] bg-background z-[200] border-l shadow-2xl flex flex-col duration-200",
          isClosing
            ? "animate-out slide-out-to-right opacity-0"
            : "animate-in slide-in-from-right",
        )}
        style={{ boxShadow: "-12px 0 48px rgba(0,0,0,0.08)" }}
      >
        {/* Header */}
        <div className="px-6 py-6 pt-8 border-b flex items-start gap-3 shrink-0">
          <div className="w-11 h-11 rounded-lg bg-primary/10 flex items-center justify-center text-sm font-bold text-primary shrink-0">
            {initials}
          </div>
          <div className="flex-1 min-w-0">
            {editingProfile ? (
              <Form {...form}>
                <form className="space-y-2">
                  <div className="flex gap-2">
                    <FormField
                      control={form.control}
                      name="first_name"
                      render={({ field }) => (
                        <FormItem className="flex-1">
                          <FormControl>
                            <Input
                              {...field}
                              placeholder="First name"
                              className="h-8 text-sm font-semibold"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="last_name"
                      render={({ field }) => (
                        <FormItem className="flex-1">
                          <FormControl>
                            <Input
                              {...field}
                              placeholder="Last name"
                              className="h-8 text-sm font-semibold"
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                  </div>
                  <FormField
                    control={form.control}
                    name="email"
                    render={({ field }) => (
                      <FormItem>
                        <FormControl>
                          <Input
                            {...field}
                            type="email"
                            placeholder="Email"
                            className="h-8 text-sm"
                          />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="home_municipality"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel className="text-xs text-muted-foreground">
                          Municipality
                        </FormLabel>
                        <MunicipalitySelect field={field} form={form} />
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <div className="flex gap-2 pt-1">
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      className="text-xs h-7"
                      onClick={() => {
                        form.reset();
                        setEditingProfile(false);
                      }}
                    >
                      Cancel
                    </Button>
                  </div>
                </form>
              </Form>
            ) : (
              <>
                <div className="font-bold text-lg leading-tight">
                  {member.first_name} {member.last_name}
                </div>
                <div className="text-xs text-muted-foreground mt-0.5">
                  {member.email}
                </div>
                <div className="flex items-center gap-2 mt-2 flex-wrap">
                  {member.home_municipality && (
                    <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
                      {member.home_municipality}
                    </span>
                  )}
                  <button
                    type="button"
                    onClick={() => setEditingProfile(true)}
                    className="inline-flex items-center justify-center w-6 h-6 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
                    title="Edit profile"
                  >
                    <Pencil className="w-3 h-3" />
                  </button>
                </div>
              </>
            )}
          </div>
          <button
            type="button"
            onClick={handleClose}
            className="shrink-0 -mt-1 inline-flex items-center justify-center w-7 h-7 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto px-6 py-5 space-y-6">
          {/* ── Role Groups ── */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
                  Role Groups
                </span>
                <Badge variant="secondary" className="text-xs px-1.5 py-0 h-4">
                  {allGroupMemberships.length}
                </Badge>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={() => {
                  setShowAddGroup((v) => !v);
                  setShowAddRole(false);
                }}
              >
                <Plus className="w-3 h-3" /> Add group
              </Button>
            </div>

            {/* Add group form */}
            {showAddGroup && (
              <div className="border rounded-lg p-3 mb-3 bg-muted/30 space-y-3 animate-in fade-in slide-in-from-top-1 duration-150">
                <p className="text-xs font-semibold text-muted-foreground">
                  Add to group
                </p>
                <select
                  value={addGroupId}
                  onChange={(e) => setAddGroupId(e.target.value)}
                  className="w-full text-sm border rounded-md px-2 py-1.5 bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                >
                  <option value="">Select group…</option>
                  {availableGroups.map((g) => (
                    <option key={g.id} value={g.id}>
                      {g.name}
                    </option>
                  ))}
                </select>
                <div>
                  <p className="text-xs text-muted-foreground mb-1">Valid</p>
                  <DateRangePicker
                    initialDateFrom={addGroupFrom}
                    initialDateTo={addGroupUntil || undefined}
                    showCompare={false}
                    align="start"
                    onUpdate={({ range }) => {
                      setAddGroupFrom(dateToStr(range.from));
                      setAddGroupUntil(range.to ? dateToStr(range.to) : "");
                    }}
                  />
                </div>
                {addGroupId && (
                  <div className="bg-background border rounded-md p-2 text-xs">
                    <p className="font-semibold text-muted-foreground uppercase tracking-wider mb-1.5">
                      Grants roles
                    </p>
                    <div className="flex flex-wrap gap-1">
                      {allGroups
                        ?.find((g) => g.id === addGroupId)
                        ?.role_names.map((r) => (
                          <RoleBadge key={r} role={r} />
                        ))}
                    </div>
                  </div>
                )}
                <div className="flex justify-end gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => {
                      setShowAddGroup(false);
                      setAddGroupId("");
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    className="h-7 text-xs"
                    disabled={!addGroupId}
                    onClick={handleAddGroup}
                  >
                    Add
                  </Button>
                </div>
              </div>
            )}

            {/* Group memberships */}
            {allGroupMemberships.length === 0 && !showAddGroup && (
              <p className="text-sm text-muted-foreground italic">
                No group memberships
              </p>
            )}

            <div className="space-y-2">
              {allGroupMemberships.map((gm) => {
                const isExpanded = expandedGroups[gm.group_id];
                const group = allGroups?.find((g) => g.id === gm.group_id);
                const dateEdit = groupDateEdits[gm.group_id];
                const currentFrom = dateEdit?.validFrom ?? gm.valid_from;
                const currentUntil =
                  dateEdit?.validUntil ?? gm.valid_until ?? "";

                return (
                  <div
                    key={`${gm.group_id}-${gm.valid_from}`}
                    className="border rounded-lg overflow-hidden"
                  >
                    <button
                      type="button"
                      onClick={() =>
                        setExpandedGroups((p) => ({
                          ...p,
                          [gm.group_id]: !p[gm.group_id],
                        }))
                      }
                      className="w-full flex items-center gap-2.5 px-3 py-2.5 text-left hover:bg-muted/40 transition-colors"
                    >
                      <Badge variant="secondary" className="font-semibold">
                        {gm.group_name}
                      </Badge>
                      <div className="flex-1" />
                      <ValidityPill
                        validFrom={gm.valid_from}
                        validUntil={gm.valid_until}
                      />
                      <ChevronDown
                        className={cn(
                          "w-4 h-4 text-muted-foreground transition-transform",
                          isExpanded && "rotate-180",
                        )}
                      />
                    </button>

                    {isExpanded && (
                      <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
                        {group && (
                          <div>
                            <p className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-1.5">
                              Inherited roles
                            </p>
                            <div className="flex flex-wrap gap-1">
                              {group.role_names.map((r) => (
                                <RoleBadge key={r} role={r} />
                              ))}
                            </div>
                          </div>
                        )}
                        <div>
                          <p className="text-xs text-muted-foreground mb-2">
                            Valid
                          </p>
                          <DateRangePicker
                            initialDateFrom={currentFrom}
                            initialDateTo={currentUntil || undefined}
                            showCompare={false}
                            align="start"
                            onUpdate={({ range }) =>
                              setGroupDateEdits((p) => ({
                                ...p,
                                [gm.group_id]: {
                                  validFrom: dateToStr(range.from),
                                  validUntil: range.to
                                    ? dateToStr(range.to)
                                    : "",
                                },
                              }))
                            }
                          />
                        </div>
                        <div className="border-t pt-2">
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 text-xs text-destructive hover:text-destructive gap-1"
                            onClick={() =>
                              handleRemoveGroup(gm.group_id, gm.valid_from)
                            }
                          >
                            <Trash2 className="w-3 h-3" /> Remove from group
                          </Button>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}

            </div>
          </section>

          <div className="h-px bg-border" />

          {/* ── Direct Roles ── */}
          <section>
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-2">
                <span className="text-xs font-bold uppercase tracking-wider text-muted-foreground">
                  Direct Roles
                </span>
                <Badge variant="secondary" className="text-xs px-1.5 py-0 h-4">
                  {Object.keys(groupedActiveRoles).length}
                </Badge>
              </div>
              <Button
                variant="outline"
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={() => {
                  setShowAddRole((v) => !v);
                  setShowAddGroup(false);
                }}
              >
                <Plus className="w-3 h-3" /> Add role
              </Button>
            </div>

            {/* Add role form */}
            {showAddRole && (
              <div className="border rounded-lg p-3 mb-3 bg-muted/30 space-y-3 animate-in fade-in slide-in-from-top-1 duration-150">
                <p className="text-xs font-semibold text-muted-foreground">
                  Add direct role
                </p>
                <select
                  value={addRoleName}
                  onChange={(e) => setAddRoleName(e.target.value)}
                  className="w-full text-sm border rounded-md px-2 py-1.5 bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                >
                  <option value="">Select role…</option>
                  {availableRoles.map((r) => (
                    <option key={r.name} value={r.name}>
                      {r.name}
                    </option>
                  ))}
                </select>
                <div>
                  <p className="text-xs text-muted-foreground mb-1">Valid</p>
                  <DateRangePicker
                    initialDateFrom={addRoleFrom}
                    initialDateTo={addRoleUntil || undefined}
                    showCompare={false}
                    align="start"
                    onUpdate={({ range }) => {
                      setAddRoleFrom(dateToStr(range.from));
                      setAddRoleUntil(range.to ? dateToStr(range.to) : "");
                    }}
                  />
                </div>
                <div className="flex justify-end gap-2">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => {
                      setShowAddRole(false);
                      setAddRoleName("");
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    className="h-7 text-xs"
                    disabled={!addRoleName}
                    onClick={handleAddRole}
                  >
                    Add
                  </Button>
                </div>
              </div>
            )}

            {activeRoles.length === 0 &&
              roleAdditions.length === 0 &&
              !showAddRole && (
                <p className="text-sm text-muted-foreground italic">
                  No directly assigned roles
                </p>
              )}

            <div className="space-y-2">
              {Object.entries(groupedActiveRoles).map(([roleName, periods]) => {
                const isExpanded = expandedRoles[roleName];
                const latest = periods[periods.length - 1];
                const previous = periods.slice(0, -1);
                const latestKey = `${latest.role_name}::${latest.valid_from}`;
                const dateEdit = roleDateEdits[latestKey];
                const currentFrom = dateEdit?.validFrom ?? latest.valid_from;
                const currentUntil =
                  dateEdit?.validUntil ?? latest.valid_until ?? "";
                const alsoFromGroup = groupRoleSet.has(roleName);

                return (
                  <div
                    key={roleName}
                    className="border rounded-lg overflow-hidden"
                  >
                    <button
                      type="button"
                      onClick={() =>
                        setExpandedRoles((p) => ({
                          ...p,
                          [roleName]: !p[roleName],
                        }))
                      }
                      className="w-full flex items-center gap-2.5 px-3 py-2.5 text-left hover:bg-muted/40 transition-colors"
                    >
                      <RoleBadge role={roleName} />
                      <div className="flex-1" />
                      <ValidityPill
                        validFrom={latest.valid_from}
                        validUntil={latest.valid_until}
                      />
                      <ChevronDown
                        className={cn(
                          "w-4 h-4 text-muted-foreground transition-transform",
                          isExpanded && "rotate-180",
                        )}
                      />
                    </button>

                    {isExpanded && (
                      <div className="border-t bg-muted/20 px-3 py-3 space-y-3">
                        {previous.map((p) => (
                          <div
                            key={p.valid_from}
                            className="flex items-center justify-between"
                          >
                            <RoleBadge role={roleName} />
                            <span className="text-xs text-muted-foreground">
                              {fmtDate(p.valid_from)} – {fmtDate(p.valid_until)}
                            </span>
                          </div>
                        ))}
                        <div className="flex items-center gap-2">
                          <span className="text-xs text-muted-foreground w-10 shrink-0">
                            Valid
                          </span>
                          <DateRangePicker
                            initialDateFrom={currentFrom}
                            initialDateTo={currentUntil || undefined}
                            showCompare={false}
                            align="start"
                            onUpdate={({ range }) =>
                              setRoleDateEdits((p) => ({
                                ...p,
                                [latestKey]: {
                                  validFrom: dateToStr(range.from),
                                  validUntil: range.to
                                    ? dateToStr(range.to)
                                    : "",
                                },
                              }))
                            }
                          />
                        </div>
                        <div className="border-t pt-2 flex items-center justify-between">
                          {alsoFromGroup && (
                            <span className="text-xs text-blue-600">
                              Also inherited from a group
                            </span>
                          )}
                          <div className="ml-auto">
                            <Button
                              variant="ghost"
                              size="sm"
                              className="h-7 text-xs text-destructive hover:text-destructive gap-1"
                              onClick={() =>
                                handleRemoveRole(
                                  latest.role_name,
                                  latest.valid_from,
                                )
                              }
                            >
                              <Trash2 className="w-3 h-3" /> Remove role
                            </Button>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}

            </div>
          </section>

          {/* ── Effective Roles ── */}
          {Object.keys(effectiveRoles).length > 0 && (
            <>
              <div className="h-px bg-border" />
              <section>
                <p className="text-xs font-bold uppercase tracking-wider text-muted-foreground mb-3">
                  Effective Roles
                </p>
                <div className="border rounded-lg p-3 bg-muted/20 space-y-2">
                  {Object.entries(effectiveRoles).map(([role, info]) => (
                    <div
                      key={role}
                      className="flex items-center justify-between gap-3"
                    >
                      <RoleBadge role={role} />
                      <span className="text-xs text-muted-foreground text-right">
                        {[
                          ...info.fromGroups,
                          ...(info.fromDirect ? ["direct"] : []),
                        ].join(", ")}
                      </span>
                    </div>
                  ))}
                </div>
              </section>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-3.5 border-t flex items-center justify-between bg-background shrink-0">
          <DeleteMembersModal
            userIds={[userId]}
            onClose={onClose}
            disabled={false}
            trigger={
              <Button
                variant="ghost"
                size="sm"
                className="text-destructive hover:text-destructive gap-1.5 text-xs"
              >
                <Trash2 className="w-3.5 h-3.5" /> Delete member
              </Button>
            }
          />
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={handleClose}>
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleSave}
              disabled={isSaving || !hasChanges}
            >
              {isSaving ? "Saving…" : "Save changes"}
            </Button>
          </div>
        </div>
      </div>
    </>
  );
}
