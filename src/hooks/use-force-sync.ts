import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { unwrapFxOutcome, unwrapPriceOutcome } from "@/lib/utils";

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
			return unwrapPriceOutcome(res.data);
		},
		onSuccess: (data) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
			toast.success(`${data.ticker} prices updated`, {
				description:
					data.added === 0
						? "Already up to date"
						: `${data.added} days updated`,
			});
		},
		onError: (e) => {
			toast.error("Price sync failed", { description: e.message });
		},
	});
}

export function useForceUpdateOneCurrencyFx() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (currency: string) => {
			const res = await commands.forceUpdateOneCurrencyFx(currency);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return unwrapFxOutcome(res.data);
		},
		onSuccess: (data) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
			toast.success(`${data.currency} FX rates updated`, {
				description:
					data.added === 0
						? "Already up to date"
						: `${data.added} days updated`,
			});
		},
		onError: (e) => {
			toast.error("FX sync failed", { description: e.message });
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
		onSuccess: (data) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
			const errorCount = data.filter((o) => o.status === "error").length;
			if (errorCount === 0) {
				toast.success("All prices updated");
			} else {
				toast.warning(
					`${data.length - errorCount} updated, ${errorCount} failed`,
					{ description: "See the results list for details" },
				);
			}
		},
		onError: (e) => {
			toast.error("Price sync failed", { description: e.message });
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
		onSuccess: (data) => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
			const errorCount = data.filter((o) => o.status === "error").length;
			if (errorCount === 0) {
				toast.success("All FX rates updated");
			} else {
				toast.warning(
					`${data.length - errorCount} updated, ${errorCount} failed`,
					{ description: "See the results list for details" },
				);
			}
		},
		onError: (e) => {
			toast.error("FX sync failed", { description: e.message });
		},
	});
}
