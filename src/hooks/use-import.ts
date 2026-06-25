import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";

export function useImportBuyCSV() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: async (csvContent: string) => {
			const res = await commands.importBuyCsv(csvContent);
			if (res.status === "error") throw new Error(getErrorMessage(res.error));
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
			toast.success("Buys imported");
		},
		onError: (e) => {
			toast.error("Buy import failed", { description: e.message });
		},
	});
}
