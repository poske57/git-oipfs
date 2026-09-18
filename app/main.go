package main

import (
	"errors"
	git "github.com/go-git/go-git/v6"
	"log/slog"
	"time"
)

func main() {
	ticker := time.NewTicker(15 * time.Minute)
	defer ticker.Stop()
	for {
		slog.Info("Start periodic reconciliation.")
		err := Reconciliation()
		if err != nil {
			slog.Error("Reconciliation failed.", "err", err.Error())
		}
		<-ticker.C
	}
}

func Reconciliation() error {
	// Check update
	repositoryPath := "../"
	repo, err := git.PlainOpen(repositoryPath)
	if err != nil {
		return err
	}

	updated, err := isRepositoryUpdated(repo)
	if err != nil {
		return err
	}

	if !updated {
		slog.Info("There are no updates to the repository.")
		return nil
	}

	// IPFS
	return nil

}

func isRepositoryUpdated(repo *git.Repository) (bool, error) {
	worktree, err := repo.Worktree()
	if err != nil {
		return false, err
	}

	// TODO: Progress をslog.Infoに表示
	err = worktree.Pull(&git.PullOptions{
		RemoteName: "origin",
	})

	if errors.Is(err, git.NoErrAlreadyUpToDate) {
		return false, nil
	}

	if err != nil {
		return false, err
	}

	return true, nil
}
