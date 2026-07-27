import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useListingMeta(mic: string, ticker: string) {
	return useQuery({
		queryKey: queryKeys.listingMeta(mic, ticker),
		queryFn: async () => {
			const res = await commands.listingMeta(ticker, mic);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		enabled: Boolean(mic && ticker),
		staleTime: 5 * 60 * 1000,
	});
}
