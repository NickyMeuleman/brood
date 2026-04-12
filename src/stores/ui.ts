import type {
	SortingState,
	Updater,
	VisibilityState,
} from "@tanstack/react-table";
import { create } from "zustand";
import type { Period } from "@/bindings";
import { columns } from "@/features/holdings/columns";

interface UIState {
	includeFees: boolean;
	displayInEur: boolean;
	period: Period;
	sorting: SortingState;
	columnVisibility: VisibilityState;
	setIncludeFees: (v: boolean) => void;
	setDisplayInEur: (v: boolean) => void;
	setPeriod: (v: Period) => void;
	setSorting: (updater: Updater<SortingState>) => void;
	setColumnVisibility: (updater: Updater<VisibilityState>) => void;
}

export const defaultColumnVisibility: VisibilityState = columns.reduce(
	(acc, col) => {
		const id = col?.id ?? (col as { accessorKey?: string }).accessorKey;
		if (id && col?.meta?.hideByDefault) {
			acc[id] = false;
		}
		return acc;
	},
	{} as VisibilityState,
);

function applyUpdater<T>(updater: Updater<T>, current: T): T {
	return typeof updater === "function"
		? (updater as (old: T) => T)(current)
		: updater;
}

export const useUIStore = create<UIState>((set, get) => ({
	includeFees: true,
	displayInEur: false,
	period: "AllTime",
	sorting: [{ id: "agg_identity", desc: false }],
	columnVisibility: defaultColumnVisibility,
	setIncludeFees: (v) => set({ includeFees: v }),
	setDisplayInEur: (v) => set({ displayInEur: v }),
	setPeriod: (v) => set({ period: v }),
	setSorting: (updater) =>
		set({ sorting: applyUpdater(updater, get().sorting) }),
	setColumnVisibility: (updater) =>
		set({ columnVisibility: applyUpdater(updater, get().columnVisibility) }),
}));
