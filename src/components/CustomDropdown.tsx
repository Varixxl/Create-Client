import React, { useState, useRef, useEffect, useMemo } from "react";
import { ChevronDown, Check, Search, X } from "lucide-react";

export interface DropdownOption {
  value: string | number;
  label: string;
  description?: string;
  recommended?: boolean;
  badge?: string;
}

interface CustomDropdownProps {
  value: string | number | null | undefined;
  options: DropdownOption[];
  onChange: (value: any) => void;
  placeholder?: string;
  disabled?: boolean;
  searchable?: boolean;
  className?: string;
}

export const CustomDropdown: React.FC<CustomDropdownProps> = React.memo(({
  value,
  options,
  onChange,
  placeholder = "Select an option...",
  disabled = false,
  searchable = false,
  className = "",
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [openUpwards, setOpenUpwards] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Close on outside click
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    }
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [isOpen]);

  // Handle escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && isOpen) {
        setIsOpen(false);
      }
    };
    if (isOpen) {
      window.addEventListener("keydown", handleKeyDown);
    }
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen]);

  // Determine open direction & focus search input
  useEffect(() => {
    if (isOpen) {
      if (containerRef.current) {
        const rect = containerRef.current.getBoundingClientRect();
        const spaceBelow = window.innerHeight - rect.bottom;
        setOpenUpwards(spaceBelow < 220 && rect.top > 220);
      }
      if (searchable) {
        setTimeout(() => {
          searchInputRef.current?.focus();
        }, 50);
      }
    } else {
      setSearch("");
    }
  }, [isOpen, searchable]);

  const selectedOption = options.find((opt) => opt.value === value);

  const filteredOptions = useMemo(() => {
    if (!searchable || !search.trim()) return options;
    const lower = search.toLowerCase();
    return options.filter((opt) =>
      opt.label.toLowerCase().includes(lower) ||
      String(opt.value).toLowerCase().includes(lower) ||
      (opt.description && opt.description.toLowerCase().includes(lower)) ||
      (opt.badge && opt.badge.toLowerCase().includes(lower))
    );
  }, [options, searchable, search]);

  return (
    <div ref={containerRef} className={`relative select-none ${className}`}>
      {/* Trigger Button */}
      <button
        type="button"
        disabled={disabled}
        onClick={() => !disabled && setIsOpen(!isOpen)}
        className={`w-full px-3 py-2.5 rounded-xl bg-[#090d14] border text-xs text-[#e4e8ee] flex items-center justify-between text-left cursor-pointer transition-all ${
          disabled ? "opacity-50 cursor-not-allowed" : "hover:border-[#3a475d]"
        } ${
          isOpen
            ? "border-[#12c3ca] shadow-[0_0_12px_rgba(18,195,202,0.15)] ring-1 ring-[#12c3ca]/40"
            : "border-[#263042]"
        }`}
      >
        <span className="truncate flex items-center gap-2">
          {selectedOption ? (
            <>
              <span className="font-medium text-[#e4e8ee]">{selectedOption.label}</span>
              {selectedOption.recommended && (
                <span className="text-[10px] font-mono font-bold text-amber-400 bg-amber-400/10 px-1.5 py-0.2 rounded border border-amber-400/30">
                  ★ Rec
                </span>
              )}
              {selectedOption.badge && (
                <span className="text-[10px] font-mono text-[#5ce1e6] bg-[#12c3ca]/10 px-1.5 py-0.2 rounded border border-[#12c3ca]/30">
                  {selectedOption.badge}
                </span>
              )}
            </>
          ) : (
            <span className="text-[#8fa0bc]">{placeholder}</span>
          )}
        </span>
        <ChevronDown
          className={`w-4 h-4 text-[#8fa0bc] transition-transform duration-200 shrink-0 ml-2 ${
            isOpen ? "rotate-180 text-[#12c3ca]" : ""
          }`}
        />
      </button>

      {/* Dropdown Menu Popover */}
      {isOpen && (
        <div
          className={`absolute z-50 left-0 right-0 rounded-xl bg-[#0b0f17] border border-[#263042] shadow-2xl overflow-hidden flex flex-col ${
            openUpwards ? "bottom-full mb-1.5" : "top-full mt-1.5"
          }`}
          style={{ maxHeight: "240px" }}
        >
          {/* Optional Search Input */}
          {searchable && (
            <div className="p-2 border-b border-[#1e2736] bg-[#0e131d] flex items-center gap-2 shrink-0">
              <Search className="w-3.5 h-3.5 text-[#8fa0bc] shrink-0" />
              <input
                ref={searchInputRef}
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Search..."
                className="w-full bg-transparent text-xs text-[#e4e8ee] placeholder-[#5a6a84] outline-none"
              />
              {search && (
                <button
                  type="button"
                  onClick={() => setSearch("")}
                  className="text-[#8fa0bc] hover:text-white p-0.5"
                >
                  <X className="w-3 h-3" />
                </button>
              )}
            </div>
          )}

          {/* Options List */}
          <div className="overflow-y-auto p-1 divide-y divide-[#18202d]/40">
            {filteredOptions.length === 0 ? (
              <div className="py-3 px-3 text-center text-xs text-[#8fa0bc]">
                No matching options found
              </div>
            ) : (
              filteredOptions.map((opt) => {
                const isSelected = opt.value === value;
                return (
                  <button
                    key={String(opt.value)}
                    type="button"
                    onClick={() => {
                      onChange(opt.value);
                      setIsOpen(false);
                    }}
                    className={`w-full px-3 py-2 text-left rounded-lg text-xs flex items-center justify-between transition-colors cursor-pointer ${
                      isSelected
                        ? "bg-[#12c3ca]/15 text-[#5ce1e6] font-bold"
                        : "text-[#c6d0df] hover:bg-[#151c27] hover:text-white"
                    }`}
                  >
                    <div className="flex flex-col min-w-0 pr-2">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="truncate">{opt.label}</span>
                        {opt.recommended && (
                          <span className="text-[10px] font-mono text-amber-400 bg-amber-400/10 px-1.5 py-0.2 rounded border border-amber-400/30">
                            ★ Recommended
                          </span>
                        )}
                        {opt.badge && (
                          <span className="text-[10px] font-mono text-[#5ce1e6] bg-[#12c3ca]/10 px-1.5 py-0.2 rounded border border-[#12c3ca]/30">
                            {opt.badge}
                          </span>
                        )}
                      </div>
                      {opt.description && (
                        <span className="text-[10px] text-[#8fa0bc] font-normal leading-tight mt-0.5">
                          {opt.description}
                        </span>
                      )}
                    </div>
                    {isSelected && (
                      <Check className="w-3.5 h-3.5 text-[#12c3ca] shrink-0" />
                    )}
                  </button>
                );
              })
            )}
          </div>
        </div>
      )}
    </div>
  );
});
