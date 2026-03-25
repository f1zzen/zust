import { useState, useEffect } from "react";
import ReactMarkdown from 'react-markdown';
import { invoke } from "@tauri-apps/api/core";
import { cn } from "@/lib/utils";
import { useModalAnimation } from "./useModalAnimation";
import { ModalBlock, ModalContent, ModalFooter, ModalHeader } from "@/Buttons";
import { Button } from "@/components/ui/button";
import { log } from "../Logic";

export const NewsModal = ({
    isOpen,
    onClose,
    updateAvailable
}: {
    isOpen: boolean;
    onClose: () => void;
    updateAvailable: string | null;
}) => {
    const [news, setNews] = useState<{ title: string; content: string } | null>(null);
    const { shouldRender, isAnimatingOut } = useModalAnimation(isOpen);

    useEffect(() => {
        if (!isOpen) return;

        const fetchNews = async () => {
            try {
                const rawUrl = "https://gist.githubusercontent.com/f1zzen/92a5742eb95e6e1922b268e9dd586f45/raw/warning";
                const response = await fetch(`${rawUrl}?t=${Date.now()}`, { cache: 'no-store' });

                if (!response.ok) return onClose();

                const text = await response.text();
                if (!text || text.trim() === "" || text.trim() === "#") return onClose();

                const lines = text.split('\n');
                const title = lines[0].replace(/^#\s*/, '').trim();
                const content = lines.slice(1).join('\n').trim();

                const currentNewsId = `news_${title}_${content}`;
                const lastSeenId = localStorage.getItem("last_seen_news_id");

                if (lastSeenId === currentNewsId && !updateAvailable) {
                    onClose();
                } else {
                    setNews({ title, content });
                }
            } catch (e) {
                log("ERR: FETCH_NEWS_FAILED: " + e);
                onClose();
            }
        };
        fetchNews();
    }, [isOpen]);

    const handleInternalClose = () => {
        if (news) {
            const currentNewsId = `news_${news.title}_${news.content}`;
            localStorage.setItem("last_seen_news_id", currentNewsId);
        }
        onClose();
    };

    if (!shouldRender || !news) return null;

    return (
        <div
            className={cn(
                "fixed inset-0 z-[1000] flex items-center justify-center bg-black/60 backdrop-blur-md transition-opacity duration-300",
                isAnimatingOut ? "opacity-0" : "opacity-100"
            )}
            onClick={handleInternalClose}
        >
            <ModalBlock onClick={(e: any) => e.stopPropagation()}>
                {updateAvailable && (
                    <div
                        className="mx-4 mt-4 p-4 bg-blue-500/20 border border-blue-500/30 rounded-2xl flex items-center justify-between cursor-pointer hover:bg-blue-500/30 transition-all active:scale-[0.98]"
                        onClick={() => invoke("open_link", { url: "https://github.com/f1zzen/zust/releases" })}
                    >
                        <div className="flex flex-col">
                            <span className="text-blue-300 text-[15px] font-bold">
                                ✨ Доступно обновление!
                            </span>
                            <span className="text-blue-200/60 text-[12px]">
                                Нажмите, чтобы скачать v{updateAvailable}
                            </span>
                        </div>
                        <span className="text-blue-400 text-xl">→</span>
                    </div>
                )}

                <ModalHeader
                    title={news.title}
                    status={updateAvailable ? `v${updateAvailable}` : "ヾ(°∇°*)"}
                    icon="🔔"
                    description="Важная информация, рекомендуем к прочтению."
                />

                <ModalContent>
                    <div className="markdown-render text-[18px] font-medium leading-relaxed text-white/90 max-h-[350px] overflow-y-auto custom-scrollbar pr-2">
                        <ReactMarkdown components={{
                            a: ({ ...props }) => (
                                <a {...props} onClick={(e) => {
                                    e.preventDefault();
                                    if (props.href) invoke("open_link", { url: props.href });
                                }} className="text-purple-400 hover:underline font-bold" />
                            ),
                            strong: ({ children }) => <strong className="text-purple-300 font-extrabold">{children}</strong>,
                            p: ({ children }) => <p className="mb-5 last:mb-0">{children}</p>,
                            h1: ({ children }) => <h1 className="text-[22px] font-bold mb-4">{children}</h1>,
                            h2: ({ children }) => <h2 className="text-[20px] font-bold mb-3">{children}</h2>,
                        }}>
                            {news.content}
                        </ReactMarkdown>
                    </div>
                </ModalContent>

                <ModalFooter>
                    <Button
                        variant="outline"
                        onClick={handleInternalClose}
                        size="sm"
                        className="w-full rounded-lg text-[12px] text-white/80"
                    >
                        Принято
                    </Button>
                </ModalFooter>
            </ModalBlock>
        </div>
    );
};