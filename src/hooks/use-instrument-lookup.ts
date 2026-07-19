import { useQuery } from "@tanstack/react-query";
import { commands } from "@/bindings";
import { isValidIsin } from "@/features/listings/shared-form";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useInstrumentLookup(isin: string) {
	return useQuery({
		queryKey: queryKeys.instrumentByIsin(isin),
		queryFn: async () => {
			const res = await commands.findInstrumentByIsin(isin);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		enabled: isValidIsin(isin),
	});
}
