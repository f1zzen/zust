'use client';

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Header } from '@/Buttons';
import {
    FileItem,
    FolderItem,
    FolderTrigger,
    FolderContent,
    Files,
    SubFiles,
} from '@/components/animate-ui/components/radix/files';

interface FileNode {
    name: string;
    is_dir: boolean;
    children?: FileNode[];
    path: string;
}

export const EditorPage = () => {
    const [loading, setLoading] = useState(true);
    const [tree, setTree] = useState<FileNode[]>([]);

    useEffect(() => { loadTree(); }, []);

    async function loadTree() {
        setLoading(true);
        try {
            const result = await invoke<FileNode[]>('get_file_tree');
            setTree(result);
        } catch (e) {
            console.error(e);
        } finally {
            setLoading(false);
        }
    }

    async function openInEditor(path: string) {
        try {
            await invoke('open_in_editor', { path });
        } catch (e) {
            console.error("Ошибка открытия:", e);
        }
    }

    const renderTree = (nodes: FileNode[]) => {
        return nodes.map((node) => {
            if (node.is_dir) {
                return (
                    <FolderItem key={node.path} value={node.name}>
                        <FolderTrigger className="w-full flex items-center cursor-pointer transition-all duration-300 ease-in-out hover:bg-[#d8b4fe] hover:text-[#1a0b2e]">
                            <span className="truncate">{node.name}</span>
                        </FolderTrigger>
                        <FolderContent>
                            <SubFiles className="ml-4">
                                {renderTree(node.children || [])}
                            </SubFiles>
                        </FolderContent>
                    </FolderItem>
                );
            }
            return (
                <FileItem
                    key={node.path}
                    onClick={() => openInEditor(node.path)}
                    className="w-full flex items-center cursor-pointer transition-all duration-300 ease-in-out hover:bg-[#d8b4fe] hover:text-[#1a0b2e] overflow-visible"
                >
                    {node.name}
                </FileItem>
            );
        });
    };

    return (
        <div className="content">
            <Header title="Редактор" />
            <div className="relative mx-auto rounded-2xl border border-white/10 bg-[#0a0514]/60 backdrop-blur-md overflow-hidden shadow-2xl w-full max-w-120">
                {loading ? (
                    <div className="p-20 flex flex-col items-center justify-center">
                        <div className="spinner mb-4"></div>
                        <span className="text-white/50 text-sm">Поиск файлов..</span>
                    </div>
                ) : (
                    <Files className="w-full p-4 overflow-y-auto max-h-125">
                        {renderTree(tree)}
                    </Files>
                )}
            </div>
        </div>
    );
};