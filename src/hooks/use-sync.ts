import { useMutation, useQueryClient } from "@tanstack/react-query";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { useSyncStore } from "@/stores/sync";
import { commands } from "../bindings";

export function useSync() {
	const queryClient = useQueryClient();
	const { setStatus, setSubmittedAt, setError } = useSyncStore();

	return useMutation({
		mutationFn: async () => {
			const res = await commands.sync();
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}
			return res.data;
		},
		onMutate: () => {
			setStatus("pending");
			setSubmittedAt(Date.now());
			setError(null);
		},
		onSuccess: () => {
			setStatus("success");
			queryClient.invalidateQueries({ queryKey: queryKeys.holdings });
			queryClient.invalidateQueries({ queryKey: queryKeys.portfolioHistory });
		},
		onError: (e) => {
			setStatus("error");
			setError(e);
		},
	});
}
