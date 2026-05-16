import { useMutation, useQueryClient } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export type ForceSyncListingArgs = {
	listing_id: number;
	exchange_mic: string;
	ticker: string;
};

export function useForceUpdateOneListingPrices() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async ({
			listing_id,
			exchange_mic,
			ticker,
		}: ForceSyncListingArgs) => {
			const res = await commands.forceUpdateOneListingPrices(
				listing_id,
				exchange_mic,
				ticker,
			);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
		},
	});
}

export function useForceUpdateOneCurrencyFx() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (currency: string) => {
			const res = await commands.forceUpdateOneCurrencyFx(currency);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
		},
	});
}

export function useForceUpdateAllPrices() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async () => {
			const res = await commands.forceUpdateAllPrices();
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
		},
	});
}

export function useForceUpdateAllFx() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async () => {
			const res = await commands.forceUpdateAllFx();
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
		},
	});
}
