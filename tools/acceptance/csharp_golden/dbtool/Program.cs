using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;

class Program
{
    static string Root;
    static Type[] allTypes;

    static object F(object o, string name)
    {
        if (o == null) return null;
        var t = o.GetType();
        var f = t.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static);
        if (f != null) return f.GetValue(o);
        var p = t.GetProperty(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static);
        if (p != null) return p.GetValue(o);
        return null;
    }

    static IEnumerable<object> Seq(object o)
    {
        if (o is IEnumerable e && !(o is string)) foreach (var x in e) yield return x;
    }

    static string S(object o) => o?.ToString() ?? "";
    static long I(object o) => o == null ? 0 : Convert.ToInt64(o);
    static string Esc(string s) => s.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", " ").Replace("\n", " ");

    static void Main(string[] args)
    {
        // argv: <serverDir> <mode> [arg]   e.g. <serverDir> export out.json
        Root = Path.GetFullPath(args.Length > 0 ? args[0] : ".") + Path.DirectorySeparatorChar;
        Directory.SetCurrentDirectory(Root);   // the original Envir resolves Server.MirDB / Server.MirADB relative to CWD
        AppDomain.CurrentDomain.AssemblyResolve += (s, e) =>
        {
            var name = new AssemblyName(e.Name).Name + ".dll";
            var p = Path.Combine(Root, name);
            return File.Exists(p) ? Assembly.LoadFrom(p) : null;
        };
        var mode = args.Length > 1 ? args[1] : "list";
        var arg2 = args.Length > 2 ? args[2] : null;
        var lib = Assembly.LoadFrom(Root + "Server.Library.dll");
        var shared = Assembly.LoadFrom(Root + "Shared.dll");
        allTypes = SafeTypes(lib).Concat(SafeTypes(shared)).ToArray();
        Console.WriteLine($"server dir: {Root}");
        Console.WriteLine($"loaded Server.Library ({allTypes.Length} types incl. Shared)");

        if (mode == "dump")
        {
            var t = allTypes.FirstOrDefault(x => x.FullName == arg2);
            if (t == null) { Console.WriteLine("not found: " + arg2); return; }
            foreach (var f in t.GetFields(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static))
                Console.WriteLine("F " + (f.IsStatic ? "static " : "") + f.FieldType.Name + " " + f.Name);
            foreach (var p in t.GetProperties(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static))
                Console.WriteLine("P " + p.PropertyType.Name + " " + p.Name);
            return;
        }
        if (mode == "list")
        {
            foreach (var t in allTypes.OrderBy(x => x.FullName)) Console.WriteLine(t.FullName);
            return;
        }
        if (mode == "export") { Export(arg2 ?? "db_export.json"); return; }
        Console.WriteLine("usage: dbtool <serverDir> list | dump <TypeFullName> | export <outfile>");
    }

    static Type[] SafeTypes(Assembly a)
    {
        try { return a.GetTypes(); }
        catch (ReflectionTypeLoadException ex) { return ex.Types.Where(t => t != null).ToArray(); }
    }

    static string ItemList(IEnumerable<object> items, int take, Dictionary<long, string> nameByIndex)
    {
        var l = new List<string>();
        foreach (var it in items.Take(take))
        {
            var ii = I(F(it, "ItemIndex"));
            l.Add($"{{\"itemIndex\":{ii},\"name\":\"{Esc(nameByIndex.TryGetValue(ii, out var n) ? n : "?")}\"," +
                  $"\"uniqueId\":{I(F(it, "UniqueID"))},\"dura\":[{I(F(it, "CurrentDura"))},{I(F(it, "MaxDura"))}],\"count\":{I(F(it, "Count"))}}}");
        }
        return "[" + string.Join(",", l) + "]";
    }

    static void Export(string outFile)
    {
        var envirType = allTypes.First(t => t.FullName == "Server.MirEnvir.Envir");
        object env = envirType.GetProperty("Main", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)?.GetValue(null)
                  ?? envirType.GetField("Main", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)?.GetValue(null)
                  ?? Activator.CreateInstance(envirType, true);
        Console.WriteLine("envir instance: " + (env == null ? "NULL" : "ok"));

        var loadDb = envirType.GetMethod("LoadDB", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        var loadAcc = envirType.GetMethod("LoadAccounts", BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance);
        Console.WriteLine("LoadDB(): " + S(loadDb?.Invoke(env, null)));
        string loadAccountsError = "";
        // The provided MirADB stores characters carrying BuffType.GameMaster, which this build's
        // Envir.GetBuffInfo() does not implement. Pre-register a BuffInfo so account loading can proceed.
        try
        {
            var bt = allTypes.First(t => t.FullName == "BuffType");
            var biType = allTypes.First(t => t.FullName == "Server.MirDatabase.BuffInfo");
            var list = (IList)F(env, "BuffInfoList");
            var existing = new HashSet<string>(Seq(list).Select(x => S(F(x, "Type"))));
            var added = 0;
            foreach (var v in Enum.GetValues(bt))
            {
                if (existing.Contains(v.ToString())) continue;
                var bi = Activator.CreateInstance(biType);
                biType.GetProperty("Type")?.SetValue(bi, v);
                list.Add(bi);
                added++;
            }
            Console.WriteLine($"pre-registered {added} BuffInfo entries (existing {existing.Count})");
        }
        catch (Exception ex) { Console.WriteLine("buff pre-register failed: " + ex.Message); }
        if (loadAcc != null)
        {
            try { loadAcc.Invoke(env, null); Console.WriteLine("LoadAccounts() done"); }
            catch (TargetInvocationException tie)
            {
                loadAccountsError = tie.InnerException?.Message ?? tie.Message;
                Console.WriteLine("LoadAccounts() FAILED: " + loadAccountsError);
            }
        }

        var itemInfos = Seq(F(env, "ItemInfoList")).ToList();
        var nameByIndex = new Dictionary<long, string>();
        foreach (var ii in itemInfos)
        {
            var idx = F(ii, "Index");
            if (idx != null) nameByIndex[I(idx)] = S(F(ii, "Name"));
        }

        var sb = new StringBuilder();
        sb.Append("{\n");
        sb.Append("  \"source\": \"" + Esc(Root) + "\",\n");
        sb.Append("  \"loadAccountsError\": \"" + Esc(loadAccountsError) + "\",\n");
        var countFields = new[] { "MapInfoList", "ItemInfoList", "MonsterInfoList", "MagicInfoList", "NPCInfoList", "QuestInfoList",
                                  "GameShopList", "RecipeInfoList", "BuffInfoList", "ConquestInfoList", "AccountList", "CharacterList",
                                  "GuildList", "HeroList", "StartItems", "GTMapList" };
        sb.Append("  \"dbCounts\": {\n");
        sb.Append(string.Join(",\n", countFields.Select(f => $"    \"{f}\": {Seq(F(env, f)).Count()}")));
        sb.Append("\n  },\n");

        var accounts = Seq(F(env, "AccountList")).ToList();
        var accLines = new List<string>();
        foreach (var a in accounts)
        {
            var chars = Seq(F(a, "Characters")).Select(c => "\"" + Esc(S(F(c, "Name"))) + "\"").ToList();
            var storage = Seq(F(a, "Storage")).Where(x => x != null).ToList();
            accLines.Add("    {\"index\":" + I(F(a, "Index")) + ",\"accountId\":\"" + Esc(S(F(a, "AccountID"))) + "\",\"userName\":\"" + Esc(S(F(a, "UserName"))) +
                         "\",\"gold\":" + I(F(a, "Gold")) + ",\"credit\":" + I(F(a, "Credit")) + ",\"admin\":" + S(F(a, "AdminAccount")).ToLower() +
                         ",\"characters\":[" + string.Join(",", chars) + "],\"storageItems\":" + storage.Count +
                         ",\"storageSample\":" + ItemList(storage, 10, nameByIndex) + "}");
        }
        sb.Append("  \"accountCount\": " + accounts.Count + ",\n  \"accounts\": [\n");
        sb.Append(string.Join(",\n", accLines));
        sb.Append("\n  ],\n");

        var chars2 = Seq(F(env, "CharacterList")).ToList();
        var charLines = new List<string>();
        foreach (var c in chars2)
        {
            var inv = Seq(F(c, "Inventory")).Where(x => x != null).ToList();
            var magics = Seq(F(c, "Magics")).Select(m => "\"" + Esc(S(F(m, "Spell"))) + "@" + S(F(m, "Level")) + "\"").ToList();
            var pets = Seq(F(c, "Pets")).Select(p => "\"" + Esc(S(F(p, "Name"))) + "\"").ToList();
            var quests = Seq(F(c, "CurrentQuests")).Select(q => I(F(q, "Index")).ToString()).ToList();
            var done = Seq(F(c, "CompletedQuests")).Select(q => I(F(q, "Index")).ToString()).ToList();
            var heroes = Seq(F(c, "Heroes")).Select(h => "\"" + Esc(S(F(h, "Name"))) + "\"").ToList();
            charLines.Add("    {\"index\":" + I(F(c, "Index")) + ",\"name\":\"" + Esc(S(F(c, "Name"))) + "\",\"level\":" + I(F(c, "Level")) +
                ",\"class\":\"" + Esc(S(F(c, "Class"))) + "\",\"gender\":\"" + Esc(S(F(c, "Gender"))) + "\",\"hair\":" + I(F(c, "Hair")) +
                ",\"mapIndex\":" + I(F(c, "CurrentMapIndex")) + ",\"loc\":\"" + Esc(S(F(c, "CurrentLocation"))) + "\",\"hp\":" + I(F(c, "HP")) +
                ",\"mp\":" + I(F(c, "MP")) + ",\"exp\":" + I(F(c, "Experience")) + ",\"pkPoints\":" + I(F(c, "PKPoints")) +
                ",\"bindMapIndex\":" + I(F(c, "BindMapIndex")) + ",\"bindLoc\":\"" + Esc(S(F(c, "BindLocation"))) + "\"" +
                ",\"lastLogin\":\"" + Esc(S(F(c, "LastLoginDate"))) + "\",\"lastLogout\":\"" + Esc(S(F(c, "LastLogoutDate"))) + "\"" +
                ",\"inventoryCount\":" + inv.Count + ",\"inventory\":" + ItemList(inv, 60, nameByIndex) +
                ",\"equipment\":" + ItemList(Seq(F(c, "Equipment")).Where(x => x != null), 20, nameByIndex) +
                ",\"questInventory\":" + ItemList(Seq(F(c, "QuestInventory")).Where(x => x != null), 20, nameByIndex) +
                ",\"magics\":[" + string.Join(",", magics) + "],\"pets\":[" + string.Join(",", pets) + "]" +
                ",\"heroes\":[" + string.Join(",", heroes) + "],\"mail\":" + Seq(F(c, "Mail")).Count() +
                ",\"currentQuests\":[" + string.Join(",", quests) + "],\"completedQuests\":[" + string.Join(",", done) + "]}");
        }
        sb.Append("  \"characterCount\": " + chars2.Count + ",\n  \"characters\": [\n");
        sb.Append(string.Join(",\n", charLines));
        sb.Append("\n  ]\n}\n");
        File.WriteAllText(outFile, sb.ToString(), new UTF8Encoding(false));
        Console.WriteLine($"wrote {outFile}; accounts={accounts.Count} characters={chars2.Count}");
    }
}
