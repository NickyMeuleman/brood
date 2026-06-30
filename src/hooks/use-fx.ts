import { useQuery } from "@tanstack/react-query";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { commands } from "../bindings";

export function useFx(date: string, currency: string) {
	return useQuery({
		enabled: Boolean(currency && date),
		queryKey: queryKeys.fxRateFor(currency, date),
		queryFn: async () => {
			const res = await commands.getRate(
				currency,
				date || new Date().toISOString(),
			);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}
			return res.data;
		},
	});
}
