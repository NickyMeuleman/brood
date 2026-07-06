import { useQuery } from "@tanstack/react-query";
import { getErrorMessage } from "@/lib/errors";
import { queryKeys } from "@/lib/queryKeys";
import { commands, type InstrumentType } from "../bindings";

export function useBrokerFee(
	broker: string,
	quantity: string,
	unitPrice: string,
	instrumentType: InstrumentType,
	mic: string,
	fxRate: string,
) {
	return useQuery({
		queryKey: queryKeys.brokerFeeFor(
			broker,
			quantity,
			unitPrice,
			instrumentType,
			mic,
			fxRate,
		),
		queryFn: async () => {
			const res = await commands.brokerFeeHint(
				broker,
				quantity,
				unitPrice,
				instrumentType,
				mic,
				fxRate,
			);
			if (res.status === "error") {
				throw new Error(getErrorMessage(res.error));
			}

			return res.data;
		},
	});
}
