import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { type CreateBrokerInput, commands } from "../bindings";

export function useAddBroker() {
	const queryClient = useQueryClient();

	return useMutation({
		mutationFn: async (input: CreateBrokerInput) => {
			const res = await commands.addBroker(input);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}
			return res.data;
		},
		onSuccess: () => {
			queryClient.invalidateQueries({ queryKey: queryKeys.brokers });
			toast.success("Broker added.");
		},
		onError: (e) => {
			toast.error("Failed to add broker.", { description: e.message });
		},
	});
}
