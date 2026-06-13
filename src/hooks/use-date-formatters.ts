import { useMemo } from "react";

export function useDateFormatters() {
	return useMemo(() => {
		return {
			weekday: new Intl.DateTimeFormat(undefined, {
				weekday: "short",
			}),
			dayMonth: new Intl.DateTimeFormat(undefined, {
				day: "numeric",
				month: "short",
			}),
			monthYear: new Intl.DateTimeFormat(undefined, {
				month: "short",
				year: "2-digit",
			}),
			fulldate: new Intl.DateTimeFormat(undefined, {
				year: "numeric",
				month: "short",
				day: "numeric",
			}),
		};
	}, []);
}
