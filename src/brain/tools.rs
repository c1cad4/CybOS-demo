use crate::CybOs;

impl CybOs {

    pub(crate) fn route_tool(&self, query: &str) -> Option<String> {
        let q = query.to_lowercase();

        // System / node status
        if q.contains("статус системы")
            || q.contains("состояние системы")
            || q.contains("статус cybos")
            || q.contains("состояние cybos")
            || q.contains("system status")
            || q.contains("cybos status")
            || q.contains("температур")
            || q.contains("батар")
        {
            return Some(self.tool_system_status());
        }

        // Recent events
        if q.contains("последние события")
            || q.contains("что произошло")
            || q.contains("события cybos")
            || q.contains("recent events")
            || q.contains("what happened")
        {
            return Some(self.tool_get_events(10));
        }

        // Entity lookup
        if q.contains("hive-")
            || q.contains("hive ")
            || q.contains("улей")
            || q.contains("пасек")
            || q.contains("apiary")
        {
            let candidates = ["Hive-003", "hive-003", "apiary-north", "hive"];

            for candidate in candidates {
                if q.contains(&candidate.to_lowercase()) {
                    return Some(self.tool_get_entity(candidate));
                }
            }

            return Some(self.tool_search_knowledge(query));
        }

        // CicadaFarm
        if q.contains("cicadafarm")
            || q.contains("цикадаферм")
            || q.contains("ферм")
            || q.contains("farm")
        {
            return Some(self.tool_farm_status());
        }

        // General knowledge search
        if q.contains("найди")
            || q.contains("поиск")
            || q.contains("информац")
            || q.contains("знаешь о")
            || q.contains("расскажи о")
            || q.contains("search")
            || q.contains("find information")
            || q.contains("what do you know about")
        {
            return Some(self.tool_search_knowledge(query));
        }

        None
    }
}
