import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { isValidIsin } from "@/features/listings/shared-form";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useListingCandidates(isin: string) {
	return useQuery({
		queryKey: queryKeys.listingCandidates(isin),
		queryFn: async () => {
			const res = await commands.findListingsByIsin(isin);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		enabled: isValidIsin(isin),
		// candidates won't change mid-session; avoid re-hitting OpenFIGI's
		// unauthenticated quota on every window refocus
		staleTime: 5 * 60 * 1000,
	});
}
