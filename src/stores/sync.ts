import { create } from "zustand";

type SyncStatus = "idle" | "pending" | "success" | "error";

interface SyncState {
	status: SyncStatus;
	submittedAt: number | null;
	error: Error | null;
	setStatus: (s: SyncStatus) => void;
	setSubmittedAt: (t: number | null) => void;
	setError: (e: Error | null) => void;
}

export const useSyncStore = create<SyncState>((set) => ({
	status: "idle",
	submittedAt: null,
	error: null,
	setStatus: (status) => set({ status }),
	setSubmittedAt: (submittedAt) => set({ submittedAt }),
	setError: (error) => set({ error }),
}));
