import { create } from "zustand";

interface UIState {
	name: string;
	setName: (value: string) => void;
}

export const useUIStore = create<UIState>((set) => ({
	name: "",
	setName: (value) => set({ name: value }),
}));
